use async_trait::async_trait;
use futures::future::join_all;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Notify;
use tracing::{info, warn};
use uuid::Uuid;
use wenyuan_core::{
    Assessment, ChatMessage, Claim, ClaimEvidenceLink, Critique, Decision, DecisionStatus,
    DeliberationMode, Evidence, EvidenceSourceKind, EvidenceTrustLevel, IdeaCard, IdeaStatus,
    Proposal, SearchBackend, SearchError, SearchResult, SeatKind, SeatModelConfig, SeatStatus,
    Session, SessionPhase, SourceSafetyFlags, ToolRun, TopicType, Vote, VoteOutcome, VotePolicy,
    build_decision, generate_merged_proposal, phase_barrier_completed,
};
use wenyuan_provider::{LlmProvider, LlmRequest, LlmResponse, ProviderError};
use wenyuan_tools::{make_tool_run, search_results_to_evidence, untrusted_evidence_notice};

#[async_trait]
pub trait ProgressSink: Send + Sync {
    async fn emit(&self, event_type: &str, payload: serde_json::Value);
}

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("provider failed for {seat:?}: {source}")]
    Provider {
        seat: SeatKind,
        source: ProviderError,
    },
    #[error("json parse failed for {seat:?} in {phase:?}: {message}")]
    Parse {
        seat: SeatKind,
        phase: SessionPhase,
        message: String,
        raw_output: String,
    },
    #[error("phase barrier failed: {0}")]
    Barrier(String),
    #[error("phase {phase:?} failed with {failures} failed seat(s)")]
    PhaseFailed {
        phase: SessionPhase,
        failures: usize,
        traces: Vec<SeatRunTrace>,
    },
    #[error("session cancelled")]
    Cancelled,
    #[error("core rule failed: {0}")]
    Core(String),
}

#[derive(Debug, Default)]
struct CancellationState {
    cancelled: AtomicBool,
    notify: Notify,
}

#[derive(Debug, Default, Clone)]
pub struct CancellationFlag(Arc<CancellationState>);

impl CancellationFlag {
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::SeqCst);
        self.0.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::SeqCst)
    }

    async fn cancelled(&self) {
        if self.is_cancelled() {
            return;
        }
        self.0.notify.notified().await;
    }
}

#[derive(Clone)]
pub struct AgentRunner {
    provider: Arc<dyn LlmProvider>,
    timeout: Duration,
    policy: VotePolicy,
    search: Option<Arc<dyn SearchBackend>>,
}

impl AgentRunner {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            provider,
            timeout: Duration::from_secs(120),
            policy: VotePolicy::default(),
            search: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_search(mut self, backend: Arc<dyn SearchBackend>) -> Self {
        self.search = Some(backend);
        self
    }

    pub fn provider(&self) -> &Arc<dyn LlmProvider> {
        &self.provider
    }

    pub async fn run_session(
        &self,
        session: Session,
        cancel: CancellationFlag,
    ) -> Result<DiscussionArtifacts, AgentError> {
        self.run_session_with_progress(session, None, cancel, None)
            .await
    }

    pub async fn run_session_with_progress(
        &self,
        mut session: Session,
        saved_artifacts: Option<DiscussionArtifacts>,
        cancel: CancellationFlag,
        progress: Option<Arc<dyn ProgressSink>>,
    ) -> Result<DiscussionArtifacts, AgentError> {
        if session.mode == DeliberationMode::SingleAgent {
            return self.run_single_agent(session, cancel, progress).await;
        }

        let mut artifacts = saved_artifacts.unwrap_or_default();
        artifacts
            .tool_runs
            .extend(session.external_tool_runs.clone());
        let model_config = session.model_config.clone();
        let mc = model_config.as_ref();
        let external_evidence = session.external_evidence.clone();

        let topic_type = self.classify_topic(&session, &cancel).await;
        let base_rules = topic_type.domain_rules().join("\n");
        let domain_rules = format!(
            "席位中文名称：谋远席、经世席、持正席。引用席位时使用中文名称。\n{}",
            base_rules
        );
        artifacts.topic_type = Some(topic_type);

        self.ensure_not_cancelled(&cancel)?;
        if artifacts.ideas.is_empty() {
            session
                .transition_to(SessionPhase::IndependentDeliberation)
                .map_err(|err| AgentError::Core(err.to_string()))?;
            artifacts
                .events
                .push("phase_started:independent_deliberation".into());
            emit_progress(
                progress.as_ref(),
                "phase_started",
                serde_json::json!({ "phase": SessionPhase::IndependentDeliberation }),
            )
            .await;
            let (ideas, traces, evidence, tool_runs) = self
                .run_independent(
                    &session,
                    &external_evidence,
                    &cancel,
                    progress.as_ref(),
                    &domain_rules,
                    session.search_enabled,
                )
                .await?;
            artifacts.ideas = ideas;
            artifacts.seat_runs.extend(traces);
            artifacts.evidence.extend(evidence);
            artifacts.tool_runs.extend(tool_runs);
            artifacts
                .events
                .push("phase_completed:independent_deliberation".into());
        } else {
            info!(
                "retry: skipping independent_deliberation ({} existing ideas)",
                artifacts.ideas.len()
            );
            session.phase = SessionPhase::CrossCritique;
        }
        emit_progress(
            progress.as_ref(),
            "phase_completed",
            serde_json::json!({ "phase": SessionPhase::IndependentDeliberation }),
        )
        .await;

        self.ensure_not_cancelled(&cancel)?;
        if artifacts.critiques.is_empty() {
            session
                .transition_to(SessionPhase::CrossCritique)
                .map_err(|err| AgentError::Core(err.to_string()))?;
            artifacts.events.push("phase_started:cross_critique".into());
            emit_progress(
                progress.as_ref(),
                "phase_started",
                serde_json::json!({ "phase": SessionPhase::CrossCritique }),
            )
            .await;
            let (critiques, traces, evidence, tool_runs) = self
                .run_critiques(
                    session.id,
                    &artifacts.ideas,
                    &cancel,
                    progress.as_ref(),
                    mc,
                    &domain_rules,
                    session.search_enabled,
                )
                .await?;
            compute_idea_statuses(&mut artifacts.ideas, &critiques, &[], None);
            artifacts.critiques = critiques;
            artifacts.seat_runs.extend(traces);
            artifacts.evidence.extend(evidence);
            artifacts.tool_runs.extend(tool_runs);
            artifacts
                .events
                .push("phase_completed:cross_critique".into());
            emit_progress(
                progress.as_ref(),
                "phase_completed",
                serde_json::json!({ "phase": SessionPhase::CrossCritique }),
            )
            .await;
        } else {
            info!(
                "retry: skipping cross_critique ({} existing critiques)",
                artifacts.critiques.len()
            );
            session.phase = SessionPhase::Revision;
        }

        self.ensure_not_cancelled(&cancel)?;
        if artifacts.proposals.is_empty() {
            session
                .transition_to(SessionPhase::Revision)
                .map_err(|err| AgentError::Core(err.to_string()))?;
            artifacts.events.push("phase_started:revision".into());
            emit_progress(
                progress.as_ref(),
                "phase_started",
                serde_json::json!({ "phase": SessionPhase::Revision }),
            )
            .await;
            let (proposals, traces, evidence, tool_runs) = self
                .run_revision(
                    session.id,
                    &artifacts.ideas,
                    &artifacts.critiques,
                    &cancel,
                    progress.as_ref(),
                    mc,
                    &domain_rules,
                    session.search_enabled,
                )
                .await?;
            compute_idea_statuses(&mut artifacts.ideas, &[], &proposals, None);
            artifacts.proposals = proposals;
            artifacts.seat_runs.extend(traces);
            artifacts.evidence.extend(evidence);
            artifacts.tool_runs.extend(tool_runs);
            artifacts.events.push("phase_completed:revision".into());
            emit_progress(
                progress.as_ref(),
                "phase_completed",
                serde_json::json!({ "phase": SessionPhase::Revision }),
            )
            .await;
        } else {
            info!(
                "retry: skipping revision ({} existing proposals)",
                artifacts.proposals.len()
            );
            session.phase = SessionPhase::Voting;
        }

        self.ensure_not_cancelled(&cancel)?;
        session
            .transition_to(SessionPhase::Voting)
            .map_err(|err| AgentError::Core(err.to_string()))?;
        artifacts.events.push("phase_started:voting".into());
        emit_progress(
            progress.as_ref(),
            "phase_started",
            serde_json::json!({ "phase": SessionPhase::Voting }),
        )
        .await;
        let (votes, traces) = self
            .run_voting(
                session.id,
                &artifacts.proposals,
                &cancel,
                progress.as_ref(),
                mc,
                &domain_rules,
            )
            .await?;
        artifacts.votes = votes;
        artifacts.seat_runs.extend(traces);
        artifacts.events.push("phase_completed:voting".into());
        emit_progress(
            progress.as_ref(),
            "phase_completed",
            serde_json::json!({ "phase": SessionPhase::Voting }),
        )
        .await;
        let mut decision_votes = artifacts.votes.clone();

        let effective_policy = session.vote_policy.clone().unwrap_or(self.policy.clone());

        match wenyuan_core::tally_votes(
            &artifacts.proposals,
            &decision_votes,
            &effective_policy,
            session.convergence_used,
        )
        .map_err(|err| AgentError::Core(err.to_string()))?
        {
            VoteOutcome::Majority(_) => {
                session
                    .transition_to(SessionPhase::Completed)
                    .map_err(|err| AgentError::Core(err.to_string()))?;
            }
            VoteOutcome::NeedsConvergence => {
                session
                    .transition_to(SessionPhase::Convergence)
                    .map_err(|err| AgentError::Core(err.to_string()))?;
                artifacts.events.push("convergence_started".into());
                emit_progress(
                    progress.as_ref(),
                    "phase_started",
                    serde_json::json!({ "phase": SessionPhase::Convergence }),
                )
                .await;

                let all_idea_ids: Vec<Uuid> = artifacts.ideas.iter().map(|i| i.id).collect();
                let merged = generate_merged_proposal(&artifacts.proposals, &all_idea_ids);
                let convergence_proposals = vec![
                    artifacts.proposals[0].clone(),
                    artifacts.proposals[1].clone(),
                    artifacts.proposals[2].clone(),
                    merged,
                ];

                let (mut second_votes, traces) = self
                    .run_voting(
                        session.id,
                        &convergence_proposals,
                        &cancel,
                        progress.as_ref(),
                        mc,
                        &domain_rules,
                    )
                    .await?;
                artifacts.seat_runs.extend(traces);
                decision_votes = second_votes.clone();
                artifacts.votes.append(&mut second_votes);
                artifacts.proposals = convergence_proposals;

                emit_progress(
                    progress.as_ref(),
                    "phase_completed",
                    serde_json::json!({ "phase": SessionPhase::Convergence }),
                )
                .await;
                session
                    .transition_to(SessionPhase::Completed)
                    .map_err(|err| AgentError::Core(err.to_string()))?;
            }
            VoteOutcome::NoMajority => {
                session
                    .transition_to(SessionPhase::Completed)
                    .map_err(|err| AgentError::Core(err.to_string()))?;
            }
        }

        let decision = build_decision(
            &artifacts.proposals,
            &decision_votes,
            &effective_policy,
            session.convergence_used,
        )
        .map_err(|err| AgentError::Core(err.to_string()))?;
        session.result = Some(decision.clone());
        artifacts.decision = Some(decision);
        compute_idea_statuses(
            &mut artifacts.ideas,
            &[],
            &artifacts.proposals,
            artifacts.decision.as_ref(),
        );
        let (claims, evidence, assessments, links) =
            extract_evidence_pool(&artifacts.ideas, &artifacts.critiques, &artifacts.proposals);
        let tool_evidence = std::mem::take(&mut artifacts.evidence);
        artifacts.claims = claims;
        artifacts.evidence = evidence;
        artifacts.evidence.extend(external_evidence);
        artifacts.evidence.extend(tool_evidence);
        artifacts.assessments = assessments;
        artifacts.claim_evidence_links = links;

        artifacts.quality = DiscussionQualityMetrics::calculate(&artifacts);

        if session.scribe_enabled {
            let scribe_result = self
                .run_scribe(&session, &artifacts, &cancel, progress.as_ref())
                .await;
            match scribe_result {
                Ok(report) => {
                    artifacts.scribe_report = Some(report);
                    artifacts.events.push("scribe_completed".into());
                }
                Err(err) => {
                    artifacts.events.push(format!("scribe_failed: {err}"));
                }
            }
        }

        artifacts.session = Some(session);
        artifacts.events.push("session_completed".into());
        Ok(artifacts)
    }

    async fn run_scribe(
        &self,
        session: &Session,
        artifacts: &DiscussionArtifacts,
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
    ) -> Result<ScribeReport, AgentError> {
        self.ensure_not_cancelled(cancel)?;

        let prompt = scribe_prompt_config();
        let input = serde_json::json!({
            "title": session.title,
            "topic": session.topic,
            "context": session.context,
            "ideas": artifacts.ideas,
            "critiques": artifacts.critiques,
            "proposals": artifacts.proposals,
            "votes": artifacts.votes,
            "decision": artifacts.decision,
        });

        let request = LlmRequest {
            session_id: session.id,
            seat: SeatKind::Mouyuan,
            phase: SessionPhase::Completed,
            repair_json: false,
            max_tokens: prompt.max_tokens,
            prompt_version: prompt.version.to_string(),
            reasoning_effort: None,
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: prompt.content.to_string(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: format!(
                        "硬性输出规则：\n- 只输出一个 JSON object。\n- 不要输出 Markdown，不要使用 ```json 代码块。\n- 字段名必须与 schema 完全一致。\n\n必须匹配的 JSON schema：\n{{\n  \"consensus_summary\": \"...\",\n  \"structural_gaps\": [\"...\"],\n  \"unresolved_conflicts\": [\"...\"],\n  \"final_report\": \"...\"\n}}\n\n输入：{}",
                        serde_json::to_string(&input).unwrap_or_else(|_| "{}".into())
                    ),
                },
            ],
            override_model: None,
        };

        emit_progress(
            progress,
            "phase_started",
            serde_json::json!({ "phase": "scribe" }),
        )
        .await;

        let (response, duration_ms) = self.call_provider(request, cancel).await;
        let response = response.map_err(|source| AgentError::Provider {
            seat: SeatKind::Mouyuan,
            source,
        })?;

        let raw = response.content;
        let report: ScribeReport = serde_json::from_str(&raw).map_err(|err| AgentError::Parse {
            seat: SeatKind::Mouyuan,
            phase: SessionPhase::Completed,
            message: format!("书记官输出解析失败: {err}"),
            raw_output: raw.clone(),
        })?;

        emit_progress(
            progress,
            "phase_completed",
            serde_json::json!({ "phase": "scribe", "duration_ms": duration_ms }),
        )
        .await;

        Ok(report)
    }

    async fn classify_topic(&self, session: &Session, cancel: &CancellationFlag) -> TopicType {
        let prompt = format!(
            "{}\n\n议题：{}",
            TopicType::classification_prompt(),
            session.topic
        );
        let request = LlmRequest {
            session_id: Uuid::nil(),
            seat: SeatKind::Mouyuan,
            phase: SessionPhase::Draft,
            messages: vec![ChatMessage {
                role: "user".into(),
                content: prompt,
            }],
            repair_json: false,
            max_tokens: 50,
            prompt_version: "topic-classification-v1".into(),
            reasoning_effort: None,
            override_model: None,
        };
        let (result, _duration_ms) = self.call_provider(request, cancel).await;
        let default = TopicType::Technical;
        match result {
            Ok(response) => {
                let text = response.content.trim().to_lowercase();
                if text.contains("personal_life") {
                    TopicType::PersonalLife
                } else if text.contains("consumer") {
                    TopicType::Consumer
                } else if text.contains("legal") {
                    TopicType::Legal
                } else if text.contains("academic") {
                    TopicType::Academic
                } else if text.contains("medical") {
                    TopicType::Medical
                } else if text.contains("financial") {
                    TopicType::Financial
                } else if text.contains("product") {
                    TopicType::Product
                } else if text.contains("strategy") {
                    TopicType::Strategy
                } else {
                    default
                }
            }
            Err(err) => {
                warn!("topic classification failed: {err}; defaulting to technical");
                default
            }
        }
    }

    async fn run_independent(
        &self,
        session: &Session,
        external_evidence: &[Evidence],
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
        domain_rules: &str,
        search_enabled: bool,
    ) -> Result<
        (
            Vec<IdeaCard>,
            Vec<SeatRunTrace>,
            Vec<Evidence>,
            Vec<ToolRun>,
        ),
        AgentError,
    > {
        let input = serde_json::json!({
            "title": session.title,
            "topic": session.topic,
            "context": session.context,
            "external_sources": external_evidence,
            "untrusted_source_notice": untrusted_evidence_notice(),
            "rule": "第一轮独议完全隔离，不引用其他席位输出。",
            "domain_rules": domain_rules,
        });
        let mc = session.model_config.as_ref();
        let run = self
            .run_three::<IndependentOutput>(
                session.id,
                SessionPhase::IndependentDeliberation,
                input,
                cancel,
                progress,
                mc,
                search_enabled,
            )
            .await?;
        let mut ideas = Vec::new();
        for (seat, output) in run.outputs {
            for idea in output.ideas.into_iter().take(5) {
                let key = normalize_for_metric(&format!("{} {}", idea.title, idea.summary));
                if let Some(existing) = ideas.iter_mut().find(|existing: &&mut IdeaCard| {
                    normalize_for_metric(&format!("{} {}", existing.title, existing.summary)) == key
                }) {
                    if !existing.source_seats.contains(&seat) {
                        existing.source_seats.push(seat);
                    }
                    continue;
                }
                ideas.push(IdeaCard {
                    id: Uuid::new_v4(),
                    proposed_by: seat,
                    source_seats: vec![seat],
                    title: idea.title,
                    summary: idea.summary,
                    value: idea.value,
                    mechanism: idea.mechanism,
                    unconventional: idea.unconventional,
                    assumptions: idea.assumptions,
                    risks: idea.risks,
                    status: IdeaStatus::Proposed,
                    challenged_by: vec![],
                    referenced_by_proposals: vec![],
                    merged_into: None,
                });
            }
        }
        Ok((ideas, run.traces, run.evidence, run.tool_runs))
    }

    async fn run_critiques(
        &self,
        session_id: Uuid,
        ideas: &[IdeaCard],
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
        model_config: Option<&HashMap<SeatKind, SeatModelConfig>>,
        domain_rules: &str,
        search_enabled: bool,
    ) -> Result<
        (
            Vec<Critique>,
            Vec<SeatRunTrace>,
            Vec<Evidence>,
            Vec<ToolRun>,
        ),
        AgentError,
    > {
        let input = serde_json::json!({
            "ideas": ideas,
            "rule": "只批议其他席位的独议结果，必须逐席给出结构化批议。",
            "domain_rules": domain_rules,
        });
        let run = self
            .run_three::<CritiqueOutput>(
                session_id,
                SessionPhase::CrossCritique,
                input,
                cancel,
                progress,
                model_config,
                search_enabled,
            )
            .await?;
        Ok((
            run.outputs
                .into_iter()
                .flat_map(|(reviewer, output)| {
                    output.reviews.into_iter().map(move |review| Critique {
                        reviewer,
                        target_seat: review.target_seat,
                        strongest_point: review.strongest_point,
                        weakest_point: review.weakest_point,
                        hidden_assumption: review.hidden_assumption,
                        challenge: review.challenge,
                        counterexample: review.counterexample,
                        suggested_improvement: review.suggested_improvement,
                        evidence_question: review.evidence_question,
                    })
                })
                .collect(),
            run.traces,
            run.evidence,
            run.tool_runs,
        ))
    }

    async fn run_revision(
        &self,
        session_id: Uuid,
        ideas: &[IdeaCard],
        critiques: &[Critique],
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
        model_config: Option<&HashMap<SeatKind, SeatModelConfig>>,
        domain_rules: &str,
        search_enabled: bool,
    ) -> Result<
        (
            Vec<Proposal>,
            Vec<SeatRunTrace>,
            Vec<Evidence>,
            Vec<ToolRun>,
        ),
        AgentError,
    > {
        let input = serde_json::json!({
            "ideas": ideas,
            "critiques": critiques,
            "rule": "形成正式策案，说明采纳、拒绝理由和置信度。",
            "domain_rules": domain_rules,
        });
        let run = self
            .run_three::<ProposalOutput>(
                session_id,
                SessionPhase::Revision,
                input,
                cancel,
                progress,
                model_config,
                search_enabled,
            )
            .await?;
        Ok((
            run.outputs
                .into_iter()
                .map(|(seat, output)| Proposal {
                    id: Uuid::new_v4(),
                    proposed_by: seat,
                    title: output.title,
                    summary: output.summary,
                    source_idea_ids: output.source_idea_ids,
                    adopted_points: output.adopted_points,
                    rejected_points: output.rejected_points,
                    rejection_reasons: output.rejection_reasons,
                    changes_from_initial: output.changes_from_initial,
                    user_value: output.user_value,
                    implementation_path: output.implementation_path,
                    risks: output.risks,
                    success_metrics: output.success_metrics,
                    confidence: output.confidence,
                })
                .collect(),
            run.traces,
            run.evidence,
            run.tool_runs,
        ))
    }

    async fn run_voting(
        &self,
        session_id: Uuid,
        proposals: &[Proposal],
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
        model_config: Option<&HashMap<SeatKind, SeatModelConfig>>,
        domain_rules: &str,
    ) -> Result<(Vec<Vote>, Vec<SeatRunTrace>), AgentError> {
        let input = serde_json::json!({
            "proposals": proposals,
            "rule": "对三个策案匿名投票，proposal_ref 按序使用 方策一、方策二、方策三。引用策案时使用方策一/二/三称呼，不要使用 proposal_0 等内部编号。",
            "domain_rules": domain_rules,
        });
        let run = self
            .run_three::<VoteOutput>(
                session_id,
                SessionPhase::Voting,
                input,
                cancel,
                progress,
                model_config,
                false,
            )
            .await?;
        let mut votes = Vec::new();
        for (voter, output) in run.outputs {
            for raw in output.votes {
                let index = parse_proposal_ref(&raw.proposal_ref);
                let Some(proposal) = proposals.get(index) else {
                    continue;
                };
                votes.push(Vote {
                    voter,
                    proposal_id: proposal.id,
                    value_score: raw.value_score as u8,
                    novelty_score: raw.novelty_score as u8,
                    feasibility_score: raw.feasibility_score as u8,
                    risk_score: raw.risk_score as u8,
                    roi_score: raw.roi_score as u8,
                    final_choice: raw.final_choice,
                    reason: raw.reason,
                    confidence: raw.confidence,
                    key_evidence: raw.key_evidence.clone(),
                    blocking_issue: raw.blocking_issue.clone(),
                });
            }
        }
        Ok((votes, run.traces))
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_one<T>(
        &self,
        session_id: Uuid,
        seat: SeatKind,
        phase: SessionPhase,
        phase_input: serde_json::Value,
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
        model_config: Option<&HashMap<SeatKind, SeatModelConfig>>,
        search_enabled: bool,
    ) -> Result<(Option<T>, Vec<SeatRunTrace>, Vec<Evidence>, Vec<ToolRun>), AgentError>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        self.ensure_not_cancelled(cancel)?;
        let result = self
            .call_and_parse::<T>(
                session_id,
                seat,
                phase,
                phase_input,
                cancel,
                progress,
                model_config,
                search_enabled,
            )
            .await?;
        Ok((
            result.parsed,
            result.traces,
            result.evidence,
            result.tool_runs,
        ))
    }

    pub async fn run_single_agent(
        &self,
        mut session: Session,
        cancel: CancellationFlag,
        progress: Option<Arc<dyn ProgressSink>>,
    ) -> Result<DiscussionArtifacts, AgentError> {
        let mut artifacts = DiscussionArtifacts::default();
        artifacts
            .tool_runs
            .extend(session.external_tool_runs.clone());
        let model_config = session.model_config.clone();
        let mc = model_config.as_ref();
        let external_evidence = session.external_evidence.clone();

        // Classify topic type and generate domain-specific rules
        let topic_type = self.classify_topic(&session, &cancel).await;
        let base_rules = topic_type.domain_rules().join("\n");
        let domain_rules = format!(
            "席位中文名称：谋远席、经世席、持正席。引用席位时使用中文名称。\n{}",
            base_rules
        );
        artifacts.topic_type = Some(topic_type);

        // Phase 1: IndependentDeliberation (single seat)
        self.ensure_not_cancelled(&cancel)?;
        session
            .transition_to(SessionPhase::IndependentDeliberation)
            .map_err(|err| AgentError::Core(err.to_string()))?;
        artifacts
            .events
            .push("phase_started:independent_deliberation".into());
        emit_progress(
            progress.as_ref(),
            "phase_started",
            serde_json::json!({ "phase": SessionPhase::IndependentDeliberation }),
        )
        .await;
        let input = serde_json::json!({
            "title": session.title,
            "topic": session.topic,
            "context": session.context,
            "external_sources": external_evidence,
            "untrusted_source_notice": untrusted_evidence_notice(),
            "rule": "第一轮独议完全隔离，不引用其他席位输出。",
            "domain_rules": domain_rules,
        });
        let (ideas_result, traces, evidence, tool_runs) = self
            .run_one::<IndependentOutput>(
                session.id,
                SeatKind::Mouyuan,
                SessionPhase::IndependentDeliberation,
                input,
                &cancel,
                progress.as_ref(),
                mc,
                session.search_enabled,
            )
            .await?;
        let mut ideas = Vec::new();
        if let Some(output) = ideas_result {
            for idea in output.ideas.into_iter().take(5) {
                ideas.push(IdeaCard {
                    id: Uuid::new_v4(),
                    proposed_by: SeatKind::Mouyuan,
                    source_seats: vec![SeatKind::Mouyuan],
                    title: idea.title,
                    summary: idea.summary,
                    value: idea.value,
                    mechanism: idea.mechanism,
                    unconventional: idea.unconventional,
                    assumptions: idea.assumptions,
                    risks: idea.risks,
                    status: IdeaStatus::Proposed,
                    challenged_by: vec![],
                    referenced_by_proposals: vec![],
                    merged_into: None,
                });
            }
        }
        artifacts.ideas = ideas;
        artifacts.seat_runs.extend(traces);
        artifacts.evidence.extend(evidence);
        artifacts.tool_runs.extend(tool_runs);
        artifacts
            .events
            .push("phase_completed:independent_deliberation".into());
        emit_progress(
            progress.as_ref(),
            "phase_completed",
            serde_json::json!({ "phase": SessionPhase::IndependentDeliberation }),
        )
        .await;

        // Phase 2: Self-critique
        self.ensure_not_cancelled(&cancel)?;
        session
            .transition_to(SessionPhase::CrossCritique)
            .map_err(|err| AgentError::Core(err.to_string()))?;
        artifacts.events.push("phase_started:self_critique".into());
        emit_progress(
            progress.as_ref(),
            "phase_started",
            serde_json::json!({ "phase": SessionPhase::CrossCritique, "mode": "self_critique" }),
        )
        .await;
        let critique_input = serde_json::json!({
            "ideas": artifacts.ideas,
            "rule": "对你自己的创意进行自我批议，逐条输出结构化批评。",
            "domain_rules": domain_rules,
        });
        let (critique_result, traces, evidence, tool_runs) = self
            .run_one::<CritiqueOutput>(
                session.id,
                SeatKind::Mouyuan,
                SessionPhase::CrossCritique,
                critique_input,
                &cancel,
                progress.as_ref(),
                mc,
                session.search_enabled,
            )
            .await?;
        let mut critiques = Vec::new();
        if let Some(output) = critique_result {
            for review in output.reviews {
                critiques.push(Critique {
                    reviewer: SeatKind::Mouyuan,
                    target_seat: SeatKind::Mouyuan,
                    strongest_point: review.strongest_point,
                    weakest_point: review.weakest_point,
                    hidden_assumption: review.hidden_assumption,
                    challenge: review.challenge,
                    counterexample: review.counterexample,
                    suggested_improvement: review.suggested_improvement,
                    evidence_question: review.evidence_question,
                });
            }
        }
        artifacts.critiques = critiques;
        artifacts.seat_runs.extend(traces);
        artifacts.evidence.extend(evidence);
        artifacts.tool_runs.extend(tool_runs);
        artifacts
            .events
            .push("phase_completed:self_critique".into());
        emit_progress(
            progress.as_ref(),
            "phase_completed",
            serde_json::json!({ "phase": SessionPhase::CrossCritique, "mode": "self_critique" }),
        )
        .await;

        // Phase 3: Revision (single seat)
        self.ensure_not_cancelled(&cancel)?;
        session
            .transition_to(SessionPhase::Revision)
            .map_err(|err| AgentError::Core(err.to_string()))?;
        artifacts.events.push("phase_started:revision".into());
        emit_progress(
            progress.as_ref(),
            "phase_started",
            serde_json::json!({ "phase": SessionPhase::Revision }),
        )
        .await;
        let revision_input = serde_json::json!({
            "ideas": artifacts.ideas,
            "critiques": artifacts.critiques,
            "rule": "根据自我批议形成最终策案，说明采纳、拒绝、修改和置信度。",
            "domain_rules": domain_rules,
        });
        let (proposal_result, traces, evidence, tool_runs) = self
            .run_one::<ProposalOutput>(
                session.id,
                SeatKind::Mouyuan,
                SessionPhase::Revision,
                revision_input,
                &cancel,
                progress.as_ref(),
                mc,
                session.search_enabled,
            )
            .await?;
        let mut proposals = Vec::new();
        if let Some(output) = proposal_result {
            proposals.push(Proposal {
                id: Uuid::new_v4(),
                proposed_by: SeatKind::Mouyuan,
                title: output.title,
                summary: output.summary,
                source_idea_ids: output.source_idea_ids,
                adopted_points: output.adopted_points,
                rejected_points: output.rejected_points,
                rejection_reasons: output.rejection_reasons,
                changes_from_initial: output.changes_from_initial,
                user_value: output.user_value,
                implementation_path: output.implementation_path,
                risks: output.risks,
                success_metrics: output.success_metrics,
                confidence: output.confidence,
            });
        }
        artifacts.proposals = proposals;
        artifacts.seat_runs.extend(traces);
        artifacts.evidence.extend(evidence);
        artifacts.tool_runs.extend(tool_runs);
        artifacts.events.push("phase_completed:revision".into());
        emit_progress(
            progress.as_ref(),
            "phase_completed",
            serde_json::json!({ "phase": SessionPhase::Revision }),
        )
        .await;

        // Completed (no voting for single agent)
        session
            .transition_to(SessionPhase::Completed)
            .map_err(|err| AgentError::Core(err.to_string()))?;

        // Build decision
        let decision = Decision {
            status: DecisionStatus::MajorityReached,
            selected_proposal: artifacts.proposals.first().cloned(),
            vote_count: 1,
            majority_reasons: vec!["单 Agent 独立产出".into()],
            minority_opinion: vec![],
            adoption_conditions: vec![],
            unresolved_questions: vec![],
            next_steps: vec![],
            self_vote_count: 0,
            minority_choices: vec![],
            reassessment_conditions: vec![],
            has_risk_blocker: false,
        };
        session.result = Some(decision.clone());
        artifacts.decision = Some(decision);
        let (claims, evidence, assessments, links) =
            extract_evidence_pool(&artifacts.ideas, &artifacts.critiques, &artifacts.proposals);
        let tool_evidence = std::mem::take(&mut artifacts.evidence);
        artifacts.claims = claims;
        artifacts.evidence = evidence;
        artifacts.evidence.extend(external_evidence);
        artifacts.evidence.extend(tool_evidence);
        artifacts.assessments = assessments;
        artifacts.claim_evidence_links = links;
        artifacts.quality = DiscussionQualityMetrics::calculate(&artifacts);

        if session.scribe_enabled {
            let scribe_result = self
                .run_scribe(&session, &artifacts, &cancel, progress.as_ref())
                .await;
            match scribe_result {
                Ok(report) => {
                    artifacts.scribe_report = Some(report);
                    artifacts.events.push("scribe_completed".into());
                }
                Err(err) => {
                    artifacts.events.push(format!("scribe_failed: {err}"));
                }
            }
        }

        artifacts.session = Some(session);
        artifacts.events.push("session_completed".into());
        Ok(artifacts)
    }

    async fn run_three<T>(
        &self,
        session_id: Uuid,
        phase: SessionPhase,
        phase_input: serde_json::Value,
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
        model_config: Option<&HashMap<SeatKind, SeatModelConfig>>,
        search_enabled: bool,
    ) -> Result<PhaseRun<T>, AgentError>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        self.ensure_not_cancelled(cancel)?;
        let futures = SeatKind::ALL.into_iter().map(|seat| {
            self.call_and_parse::<T>(
                session_id,
                seat,
                phase,
                phase_input.clone(),
                cancel,
                progress,
                model_config,
                search_enabled,
            )
        });
        let results = join_all(futures).await;
        let mut outputs = Vec::new();
        let mut traces = Vec::new();
        let mut evidence = Vec::new();
        let mut tool_runs = Vec::new();
        let mut statuses = Vec::new();
        let mut cancelled = false;

        for result in results {
            let result = match result {
                Ok(result) => result,
                Err(AgentError::Cancelled) => {
                    cancelled = true;
                    continue;
                }
                Err(err) => return Err(err),
            };
            statuses.push((
                result.seat,
                if result.parsed.is_some() {
                    SeatStatus::Completed
                } else {
                    SeatStatus::Failed
                },
            ));
            if let Some(parsed) = result.parsed {
                outputs.push((result.seat, parsed));
            }
            traces.extend(result.traces);
            evidence.extend(result.evidence);
            tool_runs.extend(result.tool_runs);
        }

        if cancelled {
            return Err(AgentError::Cancelled);
        }

        if let Err(err) = phase_barrier_completed(&statuses) {
            let failures = statuses
                .iter()
                .filter(|(_, status)| *status == SeatStatus::Failed)
                .count();
            if failures > 0 {
                return Err(AgentError::PhaseFailed {
                    phase,
                    failures,
                    traces,
                });
            }
            return Err(AgentError::Barrier(err.to_string()));
        }

        Ok(PhaseRun {
            outputs,
            traces,
            evidence,
            tool_runs,
        })
    }

    async fn call_and_parse<T>(
        &self,
        session_id: Uuid,
        seat: SeatKind,
        phase: SessionPhase,
        phase_input: serde_json::Value,
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
        model_config: Option<&HashMap<SeatKind, SeatModelConfig>>,
        search_enabled: bool,
    ) -> Result<SeatCallResult<T>, AgentError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut traces = Vec::new();
        let prompt = prompt_config(seat);

        // Search tool loop
        if search_enabled {
            if let Some(ref search_backend) = self.search {
                return self
                    .call_and_parse_with_search::<T>(
                        session_id,
                        seat,
                        phase,
                        &phase_input,
                        cancel,
                        progress,
                        model_config,
                        &prompt,
                        search_backend,
                    )
                    .await;
            }
        }

        emit_progress(
            progress,
            "seat_started",
            serde_json::json!({ "seat": seat, "phase": phase }),
        )
        .await;
        let request = self.build_request(
            session_id,
            seat,
            phase,
            false,
            &prompt,
            &phase_input,
            model_config,
            None,
            None,
        );
        let (first, first_duration_ms) = self.call_provider(request, cancel).await;
        let Ok(response) = first else {
            if matches!(first, Err(ProviderError::Cancelled)) {
                return Err(AgentError::Cancelled);
            }
            let error = first.err().unwrap_or_else(unknown_provider_error);
            emit_progress(
                progress,
                "seat_failed",
                serde_json::json!({ "seat": seat, "phase": phase, "duration_ms": first_duration_ms, "prompt_version": prompt.version, "error": error.to_string() }),
            )
            .await;
            traces.push(SeatRunTrace::failed_provider(
                TraceContext::new(session_id, seat, phase, &prompt, false, first_duration_ms),
                error,
            ));
            return Ok(SeatCallResult {
                seat,
                parsed: None,
                traces,
                evidence: Vec::new(),
                tool_runs: Vec::new(),
            });
        };

        match parse_model_json::<T>(&response.content) {
            Ok(parsed) => {
                emit_progress(
                    progress,
                    "seat_completed",
                    serde_json::json!({ "seat": seat, "phase": phase, "duration_ms": first_duration_ms, "prompt_version": prompt.version }),
                )
                .await;
                traces.push(SeatRunTrace::completed(
                    TraceContext::new(session_id, seat, phase, &prompt, false, first_duration_ms),
                    &response,
                ));
                Ok(SeatCallResult {
                    seat,
                    parsed: Some(parsed),
                    traces,
                    evidence: Vec::new(),
                    tool_runs: Vec::new(),
                })
            }
            Err(err) => {
                traces.push(SeatRunTrace::failed_parse(
                    TraceContext::new(session_id, seat, phase, &prompt, false, first_duration_ms),
                    &response,
                    err.to_string(),
                ));
                let repair_request = self.build_request(
                    session_id,
                    seat,
                    phase,
                    true,
                    &prompt,
                    &phase_input,
                    model_config,
                    Some(&err.to_string()),
                    Some(&response.content),
                );
                let (repaired, repair_duration_ms) =
                    self.call_provider(repair_request, cancel).await;
                let Ok(repaired_response) = repaired else {
                    if matches!(repaired, Err(ProviderError::Cancelled)) {
                        return Err(AgentError::Cancelled);
                    }
                    let error = repaired.err().unwrap_or_else(unknown_provider_error);
                    emit_progress(
                        progress,
                        "seat_failed",
                        serde_json::json!({ "seat": seat, "phase": phase, "duration_ms": repair_duration_ms, "prompt_version": prompt.version, "repair_attempted": true, "error": error.to_string() }),
                    )
                    .await;
                    traces.push(SeatRunTrace::failed_provider(
                        TraceContext::new(
                            session_id,
                            seat,
                            phase,
                            &prompt,
                            true,
                            repair_duration_ms,
                        ),
                        error,
                    ));
                    return Ok(SeatCallResult {
                        seat,
                        parsed: None,
                        traces,
                        evidence: Vec::new(),
                        tool_runs: Vec::new(),
                    });
                };
                match parse_model_json::<T>(&repaired_response.content) {
                    Ok(parsed) => {
                        emit_progress(
                            progress,
                            "seat_completed",
                            serde_json::json!({ "seat": seat, "phase": phase, "duration_ms": repair_duration_ms, "prompt_version": prompt.version, "repair_attempted": true }),
                        )
                        .await;
                        traces.push(SeatRunTrace::completed(
                            TraceContext::new(
                                session_id,
                                seat,
                                phase,
                                &prompt,
                                true,
                                repair_duration_ms,
                            ),
                            &repaired_response,
                        ));
                        Ok(SeatCallResult {
                            seat,
                            parsed: Some(parsed),
                            traces,
                            evidence: Vec::new(),
                            tool_runs: Vec::new(),
                        })
                    }
                    Err(err) => {
                        emit_progress(
                            progress,
                            "seat_failed",
                            serde_json::json!({ "seat": seat, "phase": phase, "duration_ms": repair_duration_ms, "prompt_version": prompt.version, "repair_attempted": true, "error": err.to_string() }),
                        )
                        .await;
                        traces.push(SeatRunTrace::failed_parse(
                            TraceContext::new(
                                session_id,
                                seat,
                                phase,
                                &prompt,
                                true,
                                repair_duration_ms,
                            ),
                            &repaired_response,
                            err.to_string(),
                        ));
                        Ok(SeatCallResult {
                            seat,
                            parsed: None,
                            traces,
                            evidence: Vec::new(),
                            tool_runs: Vec::new(),
                        })
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn call_and_parse_with_search<T>(
        &self,
        session_id: Uuid,
        seat: SeatKind,
        phase: SessionPhase,
        phase_input: &serde_json::Value,
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
        model_config: Option<&HashMap<SeatKind, SeatModelConfig>>,
        prompt: &SeatPrompt,
        search: &Arc<dyn SearchBackend>,
    ) -> Result<SeatCallResult<T>, AgentError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut traces = Vec::new();
        emit_progress(
            progress,
            "seat_started",
            serde_json::json!({ "seat": seat, "phase": phase }),
        )
        .await;

        // Build system message with search tool instruction appended
        let search_instruction = SEARCH_TOOL_INSTRUCTION;
        let system_content = format!("{}{}", prompt.content, search_instruction);

        // Build initial user message (phase input with schema)
        let schema = phase_schema(phase);
        let user_content = format!(
            "请执行当前阶段并只返回合法 JSON。\n\n硬性输出规则：\n- 只输出一个 JSON object。\n- 不要输出 Markdown，不要使用 ```json 代码块。\n- 不要输出解释性文字。\n- 字段名必须与 schema 完全一致。\n- 缺少信息时使用空字符串、空数组或 0，不要省略 schema 字段。\n\n阶段：{phase:?}\n\n必须匹配的 JSON schema 示例：\n{schema}\n\n输入：{}",
            serde_json::to_string(phase_input).unwrap_or_else(|_| "{}".into())
        );

        let mut messages = vec![
            ChatMessage {
                role: "system".into(),
                content: system_content,
            },
            ChatMessage {
                role: "user".into(),
                content: user_content,
            },
        ];

        let seat_model_config = model_config.and_then(|mc| mc.get(&seat));
        let override_model = seat_model_config.and_then(|c| c.model.clone());
        let reasoning_effort = seat_model_config.and_then(|c| c.reasoning_effort.clone());
        let max_tokens = seat_model_config
            .and_then(|c| c.max_tokens)
            .unwrap_or(prompt.max_tokens);
        let mut evidence = Vec::new();
        let mut tool_runs = Vec::new();

        for round in 0..3 {
            let request = LlmRequest {
                session_id,
                seat,
                phase,
                messages: messages.clone(),
                repair_json: false,
                max_tokens,
                prompt_version: prompt.version.to_string(),
                reasoning_effort: reasoning_effort.clone(),
                override_model: override_model.clone(),
            };
            let (result, duration_ms) = self.call_provider(request, cancel).await;
            let Ok(response) = result else {
                if matches!(result, Err(ProviderError::Cancelled)) {
                    return Err(AgentError::Cancelled);
                }
                let error = result.err().unwrap_or_else(unknown_provider_error);
                emit_progress(progress, "seat_failed", serde_json::json!({ "seat": seat, "phase": phase, "duration_ms": duration_ms, "prompt_version": prompt.version, "error": error.to_string() })).await;
                traces.push(SeatRunTrace::failed_provider(
                    TraceContext::new(session_id, seat, phase, prompt, false, duration_ms),
                    error,
                ));
                return Ok(SeatCallResult {
                    seat,
                    parsed: None,
                    traces,
                    evidence,
                    tool_runs,
                });
            };

            // Check for search tool call
            if round < 2 {
                if let Some(query) = try_extract_search_tool(&response.content) {
                    emit_progress(progress, "tool_started", serde_json::json!({ "seat": seat, "phase": phase, "tool_name": "web_search", "query": query, "round": round + 1 })).await;
                    let started = Instant::now();
                    let results_text = match search.search(&query, 5).await {
                        Ok(results) => {
                            let mut search_evidence = search_results_to_evidence(&results);
                            for item in &mut search_evidence {
                                item.proposed_by = seat;
                            }
                            let evidence_ids = search_evidence
                                .iter()
                                .map(|item| item.id)
                                .collect::<Vec<_>>();
                            let mut tool_run = make_tool_run(
                                "web_search",
                                query.clone(),
                                "completed",
                                started.elapsed(),
                                evidence_ids,
                                None,
                            );
                            tool_run.seat = Some(seat);
                            tool_run.phase = Some(phase);
                            emit_progress(progress, "tool_completed", serde_json::json!({ "seat": seat, "phase": phase, "tool_name": "web_search", "query": query, "count": search_evidence.len(), "duration_ms": tool_run.duration_ms })).await;
                            let results_text = format_search_results(&results, &query);
                            evidence.extend(search_evidence);
                            tool_runs.push(tool_run);
                            results_text
                        }
                        Err(err) => {
                            let mut tool_run = make_tool_run(
                                "web_search",
                                query.clone(),
                                "failed",
                                started.elapsed(),
                                vec![],
                                Some(err.to_string()),
                            );
                            tool_run.seat = Some(seat);
                            tool_run.phase = Some(phase);
                            emit_progress(progress, "tool_failed", serde_json::json!({ "seat": seat, "phase": phase, "tool_name": "web_search", "query": query, "duration_ms": tool_run.duration_ms, "error": err.to_string() })).await;
                            tool_runs.push(tool_run);
                            format!(
                                "搜索 [{}] 失败：{}。如果需要可换词继续输出工具调用，否则输出你的阶段输出。",
                                query, err
                            )
                        }
                    };
                    messages.push(ChatMessage {
                        role: "assistant".into(),
                        content: response.content.clone(),
                    });
                    messages.push(ChatMessage {
                        role: "user".into(),
                        content: results_text,
                    });
                    continue;
                }
            }

            // No more tool calls — try to parse as phase output
            match parse_model_json::<T>(&response.content) {
                Ok(parsed) => {
                    emit_progress(progress, "seat_completed", serde_json::json!({ "seat": seat, "phase": phase, "duration_ms": duration_ms, "prompt_version": prompt.version })).await;
                    traces.push(SeatRunTrace::completed(
                        TraceContext::new(session_id, seat, phase, prompt, false, duration_ms),
                        &response,
                    ));
                    return Ok(SeatCallResult {
                        seat,
                        parsed: Some(parsed),
                        traces,
                        evidence,
                        tool_runs,
                    });
                }
                Err(parse_err) => {
                    traces.push(SeatRunTrace::failed_parse(
                        TraceContext::new(session_id, seat, phase, prompt, false, duration_ms),
                        &response,
                        parse_err.to_string(),
                    ));
                    // Try repair
                    let repair_request = LlmRequest {
                        session_id,
                        seat,
                        phase,
                        messages: vec![
                            ChatMessage {
                                role: "system".into(),
                                content: prompt.content.to_string(),
                            },
                            ChatMessage {
                                role: "user".into(),
                                content: format!(
                                    "解析错误：{}\n上一轮原始输出：{}\n\n请只返回合法 JSON，schema 如下：\n{}",
                                    parse_err, response.content, schema
                                ),
                            },
                        ],
                        repair_json: true,
                        max_tokens,
                        prompt_version: prompt.version.to_string(),
                        reasoning_effort: reasoning_effort.clone(),
                        override_model: override_model.clone(),
                    };
                    let (repair_result, repair_ms) =
                        self.call_provider(repair_request, cancel).await;
                    let Ok(repair_response) = repair_result else {
                        if matches!(repair_result, Err(ProviderError::Cancelled)) {
                            return Err(AgentError::Cancelled);
                        }
                        let err = repair_result.err().unwrap_or_else(unknown_provider_error);
                        emit_progress(progress, "seat_failed", serde_json::json!({ "seat": seat, "phase": phase, "duration_ms": repair_ms, "prompt_version": prompt.version, "repair_attempted": true, "error": err.to_string() })).await;
                        traces.push(SeatRunTrace::failed_provider(
                            TraceContext::new(session_id, seat, phase, prompt, true, repair_ms),
                            err,
                        ));
                        return Ok(SeatCallResult {
                            seat,
                            parsed: None,
                            traces,
                            evidence,
                            tool_runs,
                        });
                    };
                    match parse_model_json::<T>(&repair_response.content) {
                        Ok(parsed) => {
                            emit_progress(progress, "seat_completed", serde_json::json!({ "seat": seat, "phase": phase, "duration_ms": repair_ms, "prompt_version": prompt.version, "repair_attempted": true })).await;
                            traces.push(SeatRunTrace::completed(
                                TraceContext::new(session_id, seat, phase, prompt, true, repair_ms),
                                &repair_response,
                            ));
                            return Ok(SeatCallResult {
                                seat,
                                parsed: Some(parsed),
                                traces,
                                evidence,
                                tool_runs,
                            });
                        }
                        Err(err) => {
                            emit_progress(progress, "seat_failed", serde_json::json!({ "seat": seat, "phase": phase, "duration_ms": repair_ms, "prompt_version": prompt.version, "repair_attempted": true, "error": err.to_string() })).await;
                            traces.push(SeatRunTrace::failed_parse(
                                TraceContext::new(session_id, seat, phase, prompt, true, repair_ms),
                                &repair_response,
                                err.to_string(),
                            ));
                            return Ok(SeatCallResult {
                                seat,
                                parsed: None,
                                traces,
                                evidence,
                                tool_runs,
                            });
                        }
                    }
                }
            }
        }

        // Exceeded max search rounds
        let error = "exceeded maximum search rounds".to_string();
        emit_progress(progress, "seat_failed", serde_json::json!({ "seat": seat, "phase": phase, "prompt_version": prompt.version, "error": error })).await;
        Err(AgentError::Core(error))
    }

    async fn call_provider(
        &self,
        request: LlmRequest,
        cancel: &CancellationFlag,
    ) -> (Result<LlmResponse, ProviderError>, u128) {
        let started = Instant::now();
        let response = tokio::select! {
            result = self.provider.complete(request) => result,
            _ = tokio::time::sleep(self.timeout) => Err(ProviderError::Timeout),
            _ = cancel.cancelled() => Err(ProviderError::Cancelled),
        };
        (response, started.elapsed().as_millis())
    }

    #[allow(clippy::too_many_arguments)]
    fn build_request(
        &self,
        session_id: Uuid,
        seat: SeatKind,
        phase: SessionPhase,
        repair_json: bool,
        prompt: &SeatPrompt,
        phase_input: &serde_json::Value,
        model_config: Option<&HashMap<SeatKind, SeatModelConfig>>,
        parse_error: Option<&str>,
        raw_output: Option<&str>,
    ) -> LlmRequest {
        let instruction = if repair_json {
            "上一轮输出不是合法 JSON。请根据同一阶段输入、解析错误和上一轮原始输出修复格式，并只返回合法 JSON。"
        } else {
            "请执行当前阶段并只返回合法 JSON。"
        };
        let repair_context = if repair_json {
            format!(
                "\n解析错误：{}\n上一轮原始输出：{}",
                parse_error.unwrap_or("未知解析错误"),
                truncate_for_repair(raw_output.unwrap_or(""))
            )
        } else {
            String::new()
        };
        let schema = phase_schema(phase);
        let seat_model_config = model_config.and_then(|mc| mc.get(&seat));
        let override_model = seat_model_config.and_then(|c| c.model.clone());
        let max_tokens = seat_model_config
            .and_then(|c| c.max_tokens)
            .unwrap_or(prompt.max_tokens);
        let reasoning_effort = seat_model_config.and_then(|c| c.reasoning_effort.clone());
        LlmRequest {
            session_id,
            seat,
            phase,
            repair_json,
            max_tokens,
            prompt_version: prompt.version.to_string(),
            reasoning_effort,
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: prompt.content.to_string(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: format!(
                        "{instruction}\n\n硬性输出规则：\n- 只输出一个 JSON object。\n- 不要输出 Markdown，不要使用 ```json 代码块。\n- 不要输出解释性文字。\n- 字段名必须与 schema 完全一致。\n- 缺少信息时使用空字符串、空数组或 0，不要省略 schema 字段。\n\n阶段：{phase:?}\n\n必须匹配的 JSON schema 示例：\n{schema}\n\n输入：{}{}",
                        serde_json::to_string(phase_input).unwrap_or_else(|_| "{}".into()),
                        repair_context
                    ),
                },
            ],
            override_model,
        }
    }

    fn ensure_not_cancelled(&self, cancel: &CancellationFlag) -> Result<(), AgentError> {
        if cancel.is_cancelled() {
            Err(AgentError::Cancelled)
        } else {
            Ok(())
        }
    }
}

const SEARCH_TOOL_INSTRUCTION: &str = "\n\n你可以搜索互联网来获取信息。如果需要搜索，输出 JSON：{\"tool\":\"search\",\"query\":\"你的搜索词\"}。收到搜索结果后，如果还需要搜索可以继续输出工具调用，否则输出你的阶段输出。";

fn try_extract_search_tool(content: &str) -> Option<String> {
    let val: serde_json::Value = serde_json::from_str(content.trim()).ok()?;
    if val.get("tool")?.as_str()? != "search" {
        return None;
    }
    let query = val.get("query")?.as_str()?;
    let trimmed = query.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn format_search_results(results: &[SearchResult], query: &str) -> String {
    if results.is_empty() {
        return format!(
            "搜索 [{}] 无结果。如果需要换词搜索请继续输出工具调用。",
            query
        );
    }
    let lines: Vec<String> = results
        .iter()
        .map(|r| format!("- {}\n  {}\n  {} ({})", r.title, r.snippet, r.url, r.source))
        .collect();
    format!(
        "搜索 [{}] 的结果：\n{}\n\n如果还需要搜索请输出工具调用 JSON，否则输出你的阶段输出。",
        query,
        lines.join("\n")
    )
}

fn phase_schema(phase: SessionPhase) -> &'static str {
    match phase {
        SessionPhase::IndependentDeliberation => {
            r#"{
  "ideas": [
    {
      "title": "",
      "summary": "",
      "value": "",
      "mechanism": "",
      "unconventional": false,
      "assumptions": [""],
      "risks": [""]
    }
  ]
}"#
        }
        SessionPhase::CrossCritique => {
            r#"{
  "reviews": [
    {
      "target_seat": "mouyuan",
      "strongest_point": "",
      "weakest_point": "",
      "challenge": "",
      "counterexample": "",
      "suggested_improvement": ""
    }
  ]
}"#
        }
        SessionPhase::Revision | SessionPhase::Convergence => {
            r#"{
  "title": "",
  "summary": "",
  "source_idea_ids": [],
  "adopted_points": [""],
  "rejection_reasons": [""],
  "user_value": "",
  "implementation_path": "",
  "risks": [""],
  "success_metrics": [""],
  "confidence": 0.0
}"#
        }
        SessionPhase::Voting => {
            r#"{
  "votes": [
    {
      "proposal_ref": "方策一",
      "value_score": 0,
      "novelty_score": 0,
      "feasibility_score": 0,
      "risk_score": 0,
      "roi_score": 0,
      "final_choice": true,
      "reason": "",
      "confidence": 0.0,
      "key_evidence": "",
      "blocking_issue": ""
    }
  ]
}"#
        }
        _ => "{}",
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscussionArtifacts {
    pub session: Option<Session>,
    pub ideas: Vec<IdeaCard>,
    pub critiques: Vec<Critique>,
    pub proposals: Vec<Proposal>,
    pub votes: Vec<Vote>,
    pub seat_runs: Vec<SeatRunTrace>,
    pub decision: Option<Decision>,
    #[serde(default)]
    pub quality: DiscussionQualityMetrics,
    #[serde(default)]
    pub claims: Vec<Claim>,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    #[serde(default)]
    pub tool_runs: Vec<ToolRun>,
    #[serde(default)]
    pub assessments: Vec<Assessment>,
    #[serde(default)]
    pub claim_evidence_links: Vec<ClaimEvidenceLink>,
    pub events: Vec<String>,
    #[serde(default)]
    pub scribe_report: Option<ScribeReport>,
    #[serde(default)]
    pub topic_type: Option<TopicType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScribeReport {
    #[serde(default)]
    pub consensus_summary: String,
    #[serde(default)]
    pub structural_gaps: Vec<String>,
    #[serde(default)]
    pub unresolved_conflicts: Vec<String>,
    #[serde(default)]
    pub final_report: String,
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscussionQualityMetrics {
    pub idea_duplicate_rate: f32,
    pub seat_similarity: f32,
    pub high_similarity_detected: bool,
    pub critique_effectiveness_rate: f32,
    pub revision_change_rate: f32,
    pub self_vote_rate: f32,
    pub vote_concentration: f32,
    pub minority_retention_rate: f32,
    pub average_tokens: f32,
    pub average_duration_ms: f32,
}

impl DiscussionQualityMetrics {
    pub fn calculate(artifacts: &DiscussionArtifacts) -> Self {
        let completed_runs: Vec<_> = artifacts
            .seat_runs
            .iter()
            .filter(|run| run.status == SeatRunStatus::Completed)
            .collect();
        let seat_similarity = average_pair_similarity(
            artifacts
                .ideas
                .iter()
                .map(|idea| format!("{} {}", idea.title, idea.summary))
                .collect(),
        );
        Self {
            idea_duplicate_rate: duplicate_rate(
                artifacts
                    .ideas
                    .iter()
                    .map(|idea| format!("{} {}", idea.title, idea.summary)),
            ),
            seat_similarity,
            high_similarity_detected: seat_similarity >= 0.75,
            critique_effectiveness_rate: ratio(
                artifacts
                    .critiques
                    .iter()
                    .filter(|critique| {
                        !critique.strongest_point.trim().is_empty()
                            && !critique.weakest_point.trim().is_empty()
                            && !critique.counterexample.trim().is_empty()
                            && !critique.suggested_improvement.trim().is_empty()
                    })
                    .count(),
                artifacts.critiques.len(),
            ),
            revision_change_rate: ratio(
                artifacts
                    .proposals
                    .iter()
                    .filter(|proposal| !proposal.rejection_reasons.is_empty())
                    .count(),
                artifacts.proposals.len(),
            ),
            self_vote_rate: artifacts
                .decision
                .as_ref()
                .map(|decision| {
                    let final_count = artifacts
                        .votes
                        .iter()
                        .filter(|vote| vote.final_choice)
                        .count();
                    if final_count > 0 {
                        ratio(decision.self_vote_count, final_count)
                    } else {
                        0.0
                    }
                })
                .unwrap_or_default(),
            vote_concentration: vote_concentration(&artifacts.votes),
            minority_retention_rate: artifacts
                .decision
                .as_ref()
                .map(|decision| ratio(decision.minority_opinion.len(), artifacts.votes.len()))
                .unwrap_or_default(),
            average_tokens: average(
                completed_runs
                    .iter()
                    .filter_map(|run| run.total_tokens.map(|tokens| tokens as f32)),
            ),
            average_duration_ms: average(completed_runs.iter().map(|run| run.duration_ms as f32)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatRunTrace {
    pub id: Uuid,
    pub session_id: Uuid,
    pub seat: SeatKind,
    pub phase: SessionPhase,
    pub status: SeatRunStatus,
    pub prompt_version: String,
    pub repair_attempted: bool,
    pub raw_output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u128,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
    pub upstream_status: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeatRunStatus {
    Completed,
    Failed,
}

impl SeatRunTrace {
    fn completed(ctx: TraceContext<'_>, response: &LlmResponse) -> Self {
        Self::from_response(SeatRunStatus::Completed, ctx, response, None)
    }

    fn failed_parse(ctx: TraceContext<'_>, response: &LlmResponse, error: String) -> Self {
        Self::from_response(SeatRunStatus::Failed, ctx, response, Some(error))
    }

    fn failed_provider(ctx: TraceContext<'_>, error: ProviderError) -> Self {
        let upstream_status = error.upstream_status();
        Self {
            id: Uuid::new_v4(),
            session_id: ctx.session_id,
            seat: ctx.seat,
            phase: ctx.phase,
            status: SeatRunStatus::Failed,
            prompt_version: ctx.prompt.version.to_string(),
            repair_attempted: ctx.repair_attempted,
            raw_output: None,
            error: Some(error.to_string()),
            duration_ms: ctx.duration_ms,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            upstream_status,
        }
    }

    fn from_response(
        status: SeatRunStatus,
        ctx: TraceContext<'_>,
        response: &LlmResponse,
        error: Option<String>,
    ) -> Self {
        let usage = response.usage.as_ref();
        Self {
            id: Uuid::new_v4(),
            session_id: ctx.session_id,
            seat: ctx.seat,
            phase: ctx.phase,
            status,
            prompt_version: ctx.prompt.version.to_string(),
            repair_attempted: ctx.repair_attempted,
            raw_output: Some(response.content.clone()),
            error,
            duration_ms: ctx.duration_ms,
            prompt_tokens: usage.map(|usage| usage.prompt_tokens),
            completion_tokens: usage.map(|usage| usage.completion_tokens),
            total_tokens: usage.map(|usage| usage.total_tokens),
            upstream_status: response.upstream_status,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TraceContext<'a> {
    session_id: Uuid,
    seat: SeatKind,
    phase: SessionPhase,
    prompt: &'a SeatPrompt,
    repair_attempted: bool,
    duration_ms: u128,
}

impl<'a> TraceContext<'a> {
    fn new(
        session_id: Uuid,
        seat: SeatKind,
        phase: SessionPhase,
        prompt: &'a SeatPrompt,
        repair_attempted: bool,
        duration_ms: u128,
    ) -> Self {
        Self {
            session_id,
            seat,
            phase,
            prompt,
            repair_attempted,
            duration_ms,
        }
    }
}

#[derive(Debug, Clone)]
struct SeatPrompt {
    version: &'static str,
    content: &'static str,
    max_tokens: u32,
}

pub fn system_prompt(seat: SeatKind) -> &'static str {
    prompt_config(seat).content
}

fn prompt_config(seat: SeatKind) -> SeatPrompt {
    match seat {
        SeatKind::Mouyuan => SeatPrompt {
            version: "mouyuan-v2",
            content: include_str!("../prompts/mouyuan-v1.md"),
            max_tokens: 32_000,
        },
        SeatKind::Jingshi => SeatPrompt {
            version: "jingshi-v2",
            content: include_str!("../prompts/jingshi-v1.md"),
            max_tokens: 32_000,
        },
        SeatKind::Chizheng => SeatPrompt {
            version: "chizheng-v2",
            content: include_str!("../prompts/chizheng-v1.md"),
            max_tokens: 32_000,
        },
    }
}

fn scribe_prompt_config() -> SeatPrompt {
    SeatPrompt {
        version: "scribe-v1",
        content: include_str!("../prompts/scribe-v1.md"),
        max_tokens: 16_000,
    }
}

struct PhaseRun<T> {
    outputs: Vec<(SeatKind, T)>,
    traces: Vec<SeatRunTrace>,
    evidence: Vec<Evidence>,
    tool_runs: Vec<ToolRun>,
}

struct SeatCallResult<T> {
    seat: SeatKind,
    parsed: Option<T>,
    traces: Vec<SeatRunTrace>,
    evidence: Vec<Evidence>,
    tool_runs: Vec<ToolRun>,
}

fn compute_idea_statuses(
    ideas: &mut [IdeaCard],
    critiques: &[Critique],
    proposals: &[Proposal],
    final_decision: Option<&Decision>,
) {
    let critique_seats: std::collections::HashSet<SeatKind> =
        critiques.iter().map(|c| c.target_seat).collect();
    for idea in ideas.iter_mut() {
        if critique_seats.contains(&idea.proposed_by) {
            idea.status = IdeaStatus::Challenged;
        }
        let critique_ids: Vec<Uuid> = critiques
            .iter()
            .filter(|c| c.target_seat == idea.proposed_by)
            .map(|_| Uuid::new_v4())
            .collect();
        idea.challenged_by = critique_ids;
    }

    for proposal in proposals {
        for idea in ideas.iter_mut() {
            if proposal.source_idea_ids.contains(&idea.id) {
                idea.status = IdeaStatus::Shortlisted;
                idea.referenced_by_proposals.push(proposal.id);
            }
            if proposal
                .rejected_points
                .iter()
                .any(|r| r.contains(&idea.title))
            {
                idea.status = IdeaStatus::Rejected;
            }
        }
    }

    if let Some(decision) = final_decision
        && let Some(selected) = &decision.selected_proposal
    {
        for idea in ideas.iter_mut() {
            if selected.source_idea_ids.contains(&idea.id) {
                idea.status = IdeaStatus::Adopted;
            }
        }
    }
}

fn extract_evidence_pool(
    ideas: &[IdeaCard],
    critiques: &[Critique],
    _proposals: &[Proposal],
) -> (
    Vec<Claim>,
    Vec<Evidence>,
    Vec<Assessment>,
    Vec<ClaimEvidenceLink>,
) {
    let mut claims = Vec::new();
    let mut evidence = Vec::new();
    let assessments = Vec::new();
    let mut links = Vec::new();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default();

    for idea in ideas {
        // Idea assumptions → Inference evidence
        for assumption in &idea.assumptions {
            let claim_id = Uuid::new_v4();
            let content_hash = Some(short_hash(assumption));
            claims.push(Claim {
                id: claim_id,
                proposed_by: idea.proposed_by,
                content: assumption.clone(),
                context: format!("Idea: {}", idea.title),
                is_supported: false,
                evidence_ids: vec![],
                assessment_ids: vec![],
                status: wenyuan_core::EvidenceStatus::Proposed,
            });
            let evidence_id = Uuid::new_v4();
            evidence.push(Evidence {
                id: evidence_id,
                proposed_by: idea.proposed_by,
                kind: wenyuan_core::EvidenceKind::Inference,
                content: assumption.clone(),
                source: format!("{} 独议", idea.proposed_by.label()),
                source_fetched_at: Some(now.clone()),
                source_hash: content_hash,
                claim_ids: vec![claim_id],
                source_kind: EvidenceSourceKind::Internal,
                trust_level: EvidenceTrustLevel::Internal,
                safety_flags: SourceSafetyFlags::default(),
            });
            links.push(ClaimEvidenceLink {
                claim_id,
                evidence_id,
                link_type: "supports".into(),
            });
        }
        // Idea risks → Claim only (challenge), no direct evidence
        for risk in &idea.risks {
            let claim_id = Uuid::new_v4();
            claims.push(Claim {
                id: claim_id,
                proposed_by: idea.proposed_by,
                content: risk.clone(),
                context: format!("Risk of Idea: {}", idea.title),
                is_supported: false,
                evidence_ids: vec![],
                assessment_ids: vec![],
                status: wenyuan_core::EvidenceStatus::Proposed,
            });
        }
        // Idea value + mechanism → Fact evidence
        if !idea.value.trim().is_empty() {
            let evidence_id = Uuid::new_v4();
            evidence.push(Evidence {
                id: evidence_id,
                proposed_by: idea.proposed_by,
                kind: wenyuan_core::EvidenceKind::Fact,
                content: idea.value.clone(),
                source: format!("{} 独议 — 用户价值", idea.proposed_by.label()),
                source_fetched_at: Some(now.clone()),
                source_hash: Some(short_hash(&idea.value)),
                claim_ids: vec![],
                source_kind: EvidenceSourceKind::Internal,
                trust_level: EvidenceTrustLevel::Internal,
                safety_flags: SourceSafetyFlags::default(),
            });
        }
        if !idea.mechanism.trim().is_empty() {
            let evidence_id = Uuid::new_v4();
            evidence.push(Evidence {
                id: evidence_id,
                proposed_by: idea.proposed_by,
                kind: wenyuan_core::EvidenceKind::Fact,
                content: idea.mechanism.clone(),
                source: format!("{} 独议 — 实现机制", idea.proposed_by.label()),
                source_fetched_at: Some(now.clone()),
                source_hash: Some(short_hash(&idea.mechanism)),
                claim_ids: vec![],
                source_kind: EvidenceSourceKind::Internal,
                trust_level: EvidenceTrustLevel::Internal,
                safety_flags: SourceSafetyFlags::default(),
            });
        }
    }

    for critique in critiques {
        if !critique.challenge.trim().is_empty() {
            let claim_id = Uuid::new_v4();
            claims.push(Claim {
                id: claim_id,
                proposed_by: critique.reviewer,
                content: critique.challenge.clone(),
                context: format!("Critique of {}", critique.target_seat.label()),
                is_supported: false,
                evidence_ids: vec![],
                assessment_ids: vec![],
                status: wenyuan_core::EvidenceStatus::Disputed,
            });
            // Link challenge to linked evidence via "challenges" type
            if !critique.evidence_question.trim().is_empty() {
                let evidence_id = Uuid::new_v4();
                evidence.push(Evidence {
                    id: evidence_id,
                    proposed_by: critique.reviewer,
                    kind: wenyuan_core::EvidenceKind::Preference,
                    content: critique.evidence_question.clone(),
                    source: format!("{} 批议 — 补证请求", critique.reviewer.label()),
                    source_fetched_at: Some(now.clone()),
                    source_hash: Some(short_hash(&critique.evidence_question)),
                    claim_ids: vec![claim_id],
                    source_kind: EvidenceSourceKind::Internal,
                    trust_level: EvidenceTrustLevel::Internal,
                    safety_flags: SourceSafetyFlags::default(),
                });
                links.push(ClaimEvidenceLink {
                    claim_id,
                    evidence_id,
                    link_type: "challenges".into(),
                });
            }
        }
        if !critique.evidence_question.trim().is_empty() {
            let evidence_id = Uuid::new_v4();
            evidence.push(Evidence {
                id: evidence_id,
                proposed_by: critique.reviewer,
                kind: wenyuan_core::EvidenceKind::Preference,
                content: critique.evidence_question.clone(),
                source: format!("{} 批议", critique.reviewer.label()),
                source_fetched_at: Some(now.clone()),
                source_hash: Some(short_hash(&critique.evidence_question)),
                claim_ids: vec![],
                source_kind: EvidenceSourceKind::Internal,
                trust_level: EvidenceTrustLevel::Internal,
                safety_flags: SourceSafetyFlags::default(),
            });
        }
    }

    // Compute is_supported: a claim is supported if it has evidence with claim_ids containing it
    for claim in &mut claims {
        claim.is_supported = evidence.iter().any(|ev| ev.claim_ids.contains(&claim.id));
        if claim.is_supported && claim.status == wenyuan_core::EvidenceStatus::Proposed {
            claim.status = wenyuan_core::EvidenceStatus::Verified;
        }
    }

    (claims, evidence, assessments, links)
}

fn short_hash(value: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn unknown_provider_error() -> ProviderError {
    ProviderError::Request("unknown provider error".to_string())
}

pub struct MockSearchBackend {
    pub results: Vec<SearchResult>,
}

impl MockSearchBackend {
    pub fn new() -> Self {
        Self {
            results: vec![
                SearchResult {
                    title: "文渊阁项目介绍".into(),
                    snippet: "文渊阁是一个本地运行的 AI 合议工作台，把同一个问题交给三个不同立场的席位分别思考、互相批议、修订方案，并通过投票形成最终结论。".into(),
                    url: "https://github.com/gordonlu/wenyuan".into(),
                    source: "mock".into(),
                },
                SearchResult {
                    title: "AI 合议与多数决机制".into(),
                    snippet: "三席合议机制包括独议、批议、复议、阁议四个阶段，支持多数决和少数留议。".into(),
                    url: "https://example.com/deliberation".into(),
                    source: "mock".into(),
                },
                SearchResult {
                    title: "三席角色设计：谋远、经世、持正".into(),
                    snippet: "谋远席负责长期战略和系统性思考，经世席关注落地路径和资源约束，持正席审查风险、伦理和边界条件。".into(),
                    url: "https://example.com/three-seats".into(),
                    source: "mock".into(),
                },
            ],
        }
    }
}

#[async_trait::async_trait]
impl SearchBackend for MockSearchBackend {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn search(&self, _query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        Ok(self.results.iter().take(limit).cloned().collect())
    }
}

async fn emit_progress(
    progress: Option<&Arc<dyn ProgressSink>>,
    event_type: &str,
    payload: serde_json::Value,
) {
    if let Some(progress) = progress {
        progress.emit(event_type, payload).await;
    }
}

fn ratio(numerator: usize, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

fn average(values: impl Iterator<Item = f32>) -> f32 {
    let mut sum = 0.0;
    let mut count = 0usize;
    for value in values {
        sum += value;
        count += 1;
    }
    ratio_float(sum, count)
}

fn ratio_float(sum: f32, count: usize) -> f32 {
    if count == 0 { 0.0 } else { sum / count as f32 }
}

/// Parse a proposal reference string to a zero-based index.
/// Supports both new Chinese format (方策一/二/三) and old format (proposal_0/1/2).
fn parse_proposal_ref(ref_str: &str) -> usize {
    match ref_str {
        "方策一" | "方案一" | "proposal_0" => 0,
        "方策二" | "方案二" | "proposal_1" => 1,
        "方策三" | "方案三" | "proposal_2" => 2,
        _ => ref_str
            .trim_start_matches("proposal_")
            .parse::<usize>()
            .unwrap_or(0),
    }
}

/// Clean a raw LLM response into a usable query string.
/// Handles JSON {"query":"..."}, markdown code fences, and common prefixes.
#[cfg(test)]
fn clean_search_query(raw: &str) -> String {
    let s = raw.trim();
    // Try direct JSON parse first
    if let Some(q) = try_extract_json_query(s) {
        return q;
    }
    // Strip common prefixes
    let mut s = s;
    if let Some(rest) = s.strip_prefix("关键词：") {
        s = rest;
    }
    if let Some(rest) = s.strip_prefix("关键词:") {
        s = rest;
    }
    if let Some(rest) = s.strip_prefix("Keywords:") {
        s = rest;
    }
    if let Some(rest) = s.strip_prefix("keywords:") {
        s = rest;
    }
    // Remove markdown code fences and try JSON again
    let s = s
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let Some(q) = try_extract_json_query(s) {
        return q;
    }
    // Take only the first line if multiple lines
    let s = s.lines().next().unwrap_or(s).trim().to_string();
    s
}

/// Try to extract the "query" field from a JSON value string.
#[cfg(test)]
fn try_extract_json_query(s: &str) -> Option<String> {
    let val: serde_json::Value = serde_json::from_str(s).ok()?;
    let q = val.get("query")?.as_str()?;
    let trimmed = q.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Check whether a query string is usable for search.
/// Rejects single-char queries (including Chinese), empty strings, and whitespace-only strings.
#[cfg(test)]
fn valid_search_query(query: &str) -> bool {
    let non_ws_count = query.chars().filter(|c| !c.is_whitespace()).count();
    non_ws_count >= 3
}

fn duplicate_rate(values: impl Iterator<Item = String>) -> f32 {
    let mut seen = HashSet::new();
    let mut total = 0usize;
    let mut duplicates = 0usize;
    for value in values {
        total += 1;
        if !seen.insert(normalize_for_metric(&value)) {
            duplicates += 1;
        }
    }
    ratio(duplicates, total)
}

fn average_pair_similarity(values: Vec<String>) -> f32 {
    let normalized: Vec<_> = values
        .iter()
        .map(|value| token_set(value))
        .filter(|tokens| !tokens.is_empty())
        .collect();
    let mut sum = 0.0;
    let mut pairs = 0usize;
    for left in 0..normalized.len() {
        for right in (left + 1)..normalized.len() {
            let intersection = normalized[left].intersection(&normalized[right]).count() as f32;
            let union = normalized[left].union(&normalized[right]).count() as f32;
            if union > 0.0 {
                sum += intersection / union;
                pairs += 1;
            }
        }
    }
    ratio_float(sum, pairs)
}

fn vote_concentration(votes: &[Vote]) -> f32 {
    let mut counts = std::collections::HashMap::new();
    let mut total = 0usize;
    for vote in votes.iter().filter(|vote| vote.final_choice) {
        total += 1;
        *counts.entry(vote.proposal_id).or_insert(0usize) += 1;
    }
    ratio(counts.values().copied().max().unwrap_or_default(), total)
}

fn normalize_for_metric(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
        .flat_map(char::to_lowercase)
        .collect()
}

fn truncate_for_repair(value: &str) -> String {
    const MAX_CHARS: usize = 6000;
    let mut output = String::new();
    for ch in value.chars().take(MAX_CHARS) {
        output.push(ch);
    }
    if value.chars().count() > MAX_CHARS {
        output.push_str("\n...[truncated]");
    }
    output
}

fn parse_model_json<T>(content: &str) -> Result<T, serde_json::Error>
where
    T: for<'de> Deserialize<'de>,
{
    let cleaned = clean_json_string(content);
    serde_json::from_str(&cleaned)
}

/// Attempt to fix common LLM JSON issues: markdown fences, extra text, trailing commas,
/// single quotes, truncated content.
fn clean_json_string(raw: &str) -> String {
    let s = strip_markdown_json(raw).trim().to_string();

    // Try direct parse first
    if serde_json::from_str::<serde_json::Value>(&s).is_ok() {
        return s;
    }

    // Find the outermost JSON object ({...})
    let Some(start) = s.find('{') else {
        return s;
    };
    let tail = &s[start..];

    // Find matching closing brace via brace counter
    let mut depth = 0u32;
    let end = tail
        .char_indices()
        .find(|&(_, c)| {
            match c {
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            depth == 0
        })
        .map(|(i, _)| i + 1)
        .unwrap_or(tail.len());
    let mut cleaned = tail[..end].to_string();

    // Remove trailing commas before } or ] (do iteratively until stable)
    loop {
        let before = cleaned.clone();
        cleaned = cleaned
            .replace(",\n}", "\n}")
            .replace(",}", "}")
            .replace(",\n]", "\n]")
            .replace(",]", "]");
        if cleaned == before {
            break;
        }
    }

    // If still invalid, try replacing single quotes with double quotes
    if serde_json::from_str::<serde_json::Value>(&cleaned).is_err() && cleaned.contains('\'') {
        let mut in_single = false;
        let mut out = String::with_capacity(cleaned.len());
        for ch in cleaned.chars() {
            match ch {
                '\'' => {
                    in_single = !in_single;
                    out.push('"');
                }
                _ => out.push(ch),
            }
        }
        cleaned = out;
    }

    cleaned
}

fn strip_markdown_json(content: &str) -> &str {
    let trimmed = content.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    let rest = rest
        .strip_prefix("json")
        .or_else(|| rest.strip_prefix("JSON"))
        .unwrap_or(rest)
        .trim_start_matches(|ch: char| ch.is_whitespace());
    rest.strip_suffix("```").map(str::trim).unwrap_or(rest)
}

fn deserialize_uuid_vec_lossy<'de, D>(deserializer: D) -> Result<Vec<Uuid>, D::Error>
where
    D: Deserializer<'de>,
{
    let values = Vec::<String>::deserialize(deserializer)?;
    Ok(values
        .into_iter()
        .filter_map(|value| Uuid::parse_str(value.trim()).ok())
        .collect())
}

/// Deserialize a string field that may be a JSON string, an array of strings (take first),
/// or null/missing (default to empty string).
fn deserialize_string_loose<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::String(s) => s,
        serde_json::Value::Array(arr) => arr
            .first()
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default(),
        _ => String::new(),
    })
}

fn deserialize_boolish<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::Bool(value) => value,
        serde_json::Value::Number(value) => value.as_i64().unwrap_or_default() != 0,
        serde_json::Value::String(value) => {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "true" | "yes" | "y" | "support" | "supported" | "approve" | "approved" | "1"
            )
        }
        _ => false,
    })
}

fn token_set(value: &str) -> HashSet<String> {
    let mut tokens = HashSet::new();
    for token in value.split(|ch: char| ch.is_whitespace() || ch.is_ascii_punctuation()) {
        let normalized = normalize_for_metric(token);
        if !normalized.is_empty() {
            tokens.insert(normalized);
        }
    }
    let chars: Vec<char> = value
        .chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
        .collect();
    for window in chars.windows(2) {
        tokens.insert(window.iter().collect::<String>().to_lowercase());
    }
    tokens
}

#[derive(Debug, Deserialize)]
struct IndependentOutput {
    ideas: Vec<RawIdea>,
}

#[derive(Debug, Deserialize)]
struct RawIdea {
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    title: String,
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    summary: String,
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    value: String,
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    mechanism: String,
    #[serde(default)]
    unconventional: bool,
    #[serde(default)]
    assumptions: Vec<String>,
    #[serde(default)]
    risks: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CritiqueOutput {
    reviews: Vec<RawCritique>,
}

#[derive(Debug, Deserialize)]
struct RawCritique {
    #[serde(alias = "seat")]
    target_seat: SeatKind,
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    strongest_point: String,
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    weakest_point: String,
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    hidden_assumption: String,
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    challenge: String,
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    counterexample: String,
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    suggested_improvement: String,
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    evidence_question: String,
}

#[derive(Debug, Deserialize)]
struct ProposalOutput {
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    title: String,
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    summary: String,
    #[serde(default, deserialize_with = "deserialize_uuid_vec_lossy")]
    source_idea_ids: Vec<Uuid>,
    #[serde(default)]
    adopted_points: Vec<String>,
    #[serde(default)]
    rejected_points: Vec<String>,
    #[serde(default)]
    rejection_reasons: Vec<String>,
    #[serde(default)]
    changes_from_initial: Vec<String>,
    #[serde(
        default,
        alias = "value",
        alias = "user_benefit",
        deserialize_with = "deserialize_string_loose"
    )]
    user_value: String,
    #[serde(
        default,
        alias = "implementation_plan",
        alias = "action_plan",
        deserialize_with = "deserialize_string_loose"
    )]
    implementation_path: String,
    #[serde(default)]
    risks: Vec<String>,
    #[serde(default, alias = "metrics", alias = "success_criteria")]
    success_metrics: Vec<String>,
    #[serde(default)]
    confidence: f32,
}

#[derive(Debug, Deserialize)]
struct VoteOutput {
    votes: Vec<RawVote>,
}

#[derive(Debug, Deserialize)]
struct RawVote {
    #[serde(
        default,
        alias = "proposal_id",
        alias = "proposal",
        alias = "choice",
        deserialize_with = "deserialize_string_loose"
    )]
    proposal_ref: String,
    #[serde(default)]
    value_score: f32,
    #[serde(default)]
    novelty_score: f32,
    #[serde(default)]
    feasibility_score: f32,
    #[serde(default)]
    risk_score: f32,
    #[serde(default)]
    roi_score: f32,
    #[serde(default, deserialize_with = "deserialize_boolish")]
    final_choice: bool,
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    reason: String,
    #[serde(default)]
    confidence: f32,
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    key_evidence: String,
    #[serde(default, deserialize_with = "deserialize_string_loose")]
    blocking_issue: String,
}

pub fn has_duplicate_phase_start(events: &[String], phase: &str) -> bool {
    let mut seen = HashSet::new();
    events
        .iter()
        .filter(|event| event.as_str() == format!("phase_started:{phase}"))
        .any(|event| !seen.insert(event))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Mutex;
    use wenyuan_core::DecisionStatus;
    use wenyuan_provider::{MockProvider, MockScenario};

    struct HttpStatusProvider;

    #[async_trait]
    impl LlmProvider for HttpStatusProvider {
        async fn complete(&self, _request: LlmRequest) -> Result<LlmResponse, ProviderError> {
            Err(ProviderError::HttpStatus {
                status: 429,
                message: "rate limited".into(),
            })
        }
    }

    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl ProgressSink for RecordingSink {
        async fn emit(&self, event_type: &str, _payload: serde_json::Value) {
            self.events.lock().await.push(event_type.to_string());
        }
    }

    fn runner(scenario: MockScenario) -> AgentRunner {
        AgentRunner::new(Arc::new(MockProvider::new(scenario)))
    }

    #[tokio::test]
    async fn full_mock_discussion_reaches_majority() {
        let artifacts = runner(MockScenario::SuccessMajority)
            .run_session(
                Session::new("title", "topic", ""),
                CancellationFlag::default(),
            )
            .await
            .unwrap();
        assert!(artifacts.decision.unwrap().selected_proposal.is_some());
        assert_eq!(artifacts.ideas.len(), 6);
        assert!(
            artifacts
                .seat_runs
                .iter()
                .all(|run| !run.prompt_version.is_empty())
        );
        assert!(
            artifacts
                .seat_runs
                .iter()
                .all(|run| run.total_tokens == Some(200))
        );
        assert!(artifacts.ideas.iter().any(|idea| idea.unconventional));
        assert!(!artifacts.quality.high_similarity_detected);
        assert_eq!(artifacts.quality.critique_effectiveness_rate, 1.0);
        assert_eq!(artifacts.quality.revision_change_rate, 1.0);
        assert!(artifacts.quality.average_tokens > 0.0);
    }

    #[tokio::test]
    async fn progress_sink_receives_phase_and_seat_events() {
        let sink = Arc::new(RecordingSink::default());
        runner(MockScenario::SuccessMajority)
            .run_session_with_progress(
                Session::new("title", "topic", ""),
                None,
                CancellationFlag::default(),
                Some(sink.clone()),
            )
            .await
            .unwrap();

        let events = sink.events.lock().await;
        assert!(events.iter().any(|event| event == "phase_started"));
        assert!(events.iter().any(|event| event == "phase_completed"));
        assert!(events.iter().any(|event| event == "seat_started"));
        assert!(events.iter().any(|event| event == "seat_completed"));
    }

    #[test]
    fn prompt_versions_are_seat_specific() {
        let prompts = SeatKind::ALL.map(prompt_config);
        assert_eq!(prompts[0].version, "mouyuan-v2");
        assert_eq!(prompts[1].max_tokens, 32_000);
        assert!(prompts[2].content.contains("持正席"));
    }

    #[tokio::test]
    async fn parse_failure_is_repaired_once() {
        let artifacts = runner(MockScenario::MalformedThenRepair)
            .run_session(
                Session::new("title", "topic", ""),
                CancellationFlag::default(),
            )
            .await
            .unwrap();
        assert!(artifacts.decision.is_some());
    }

    #[tokio::test]
    async fn split_then_convergence_keeps_no_majority_when_second_vote_splits() {
        let artifacts = runner(MockScenario::SplitThenConvergence)
            .run_session(
                Session::new("title", "topic", ""),
                CancellationFlag::default(),
            )
            .await
            .unwrap();
        let decision = artifacts.decision.unwrap();
        assert_eq!(decision.status, DecisionStatus::NoMajority);
        assert!(decision.selected_proposal.is_none());
        assert!(
            artifacts
                .events
                .iter()
                .any(|event| event == "convergence_started")
        );
        assert_eq!(artifacts.votes.len(), 6);
    }

    #[tokio::test]
    async fn twenty_mock_discussions_complete_without_deadlock() {
        for index in 0..20 {
            let artifacts = runner(MockScenario::SuccessMajority)
                .run_session(
                    Session::new(format!("title {index}"), "topic", ""),
                    CancellationFlag::default(),
                )
                .await
                .unwrap();
            assert_eq!(
                artifacts.decision.unwrap().status,
                DecisionStatus::MajorityReached
            );
        }
    }

    #[tokio::test]
    async fn repeated_parse_failure_records_error() {
        let err = runner(MockScenario::AlwaysMalformed)
            .run_session(
                Session::new("title", "topic", ""),
                CancellationFlag::default(),
            )
            .await
            .unwrap_err();
        let AgentError::PhaseFailed { traces, .. } = err else {
            panic!("expected phase failure");
        };
        assert!(traces.iter().any(|trace| {
            trace.status == SeatRunStatus::Failed
                && trace.raw_output.as_deref() == Some("{ broken json")
        }));
    }

    #[tokio::test]
    async fn single_seat_failure_does_not_advance() {
        let err = runner(MockScenario::SingleSeatFailure)
            .run_session(
                Session::new("title", "topic", ""),
                CancellationFlag::default(),
            )
            .await
            .unwrap_err();
        let AgentError::PhaseFailed { traces, .. } = err else {
            panic!("expected phase failure");
        };
        assert!(traces.iter().any(|trace| {
            trace.seat == SeatKind::Jingshi
                && trace.status == SeatRunStatus::Failed
                && trace.error.as_deref()
                    == Some("provider request failed: mock single seat failure")
        }));
    }

    #[tokio::test]
    async fn single_seat_timeout_is_recorded_as_failed_trace() {
        let err = runner(MockScenario::SingleSeatTimeout)
            .run_session(
                Session::new("title", "topic", ""),
                CancellationFlag::default(),
            )
            .await
            .unwrap_err();
        let AgentError::PhaseFailed { traces, .. } = err else {
            panic!("expected phase failure");
        };
        assert!(traces.iter().any(|trace| {
            trace.seat == SeatKind::Jingshi
                && trace.status == SeatRunStatus::Failed
                && trace.error.as_deref() == Some("provider timeout")
        }));
    }

    #[tokio::test]
    async fn running_provider_call_can_be_cancelled() {
        let cancel = CancellationFlag::default();
        let cancel_task = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(25)).await;
            cancel_task.cancel();
        });

        let started = Instant::now();
        let err = AgentRunner::new(Arc::new(MockProvider::new(MockScenario::Timeout)))
            .with_timeout(Duration::from_secs(5))
            .run_session(Session::new("title", "topic", ""), cancel)
            .await
            .unwrap_err();

        assert!(matches!(err, AgentError::Cancelled));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn upstream_status_is_recorded_for_provider_failures() {
        let err = AgentRunner::new(Arc::new(HttpStatusProvider))
            .run_session(
                Session::new("title", "topic", ""),
                CancellationFlag::default(),
            )
            .await
            .unwrap_err();
        let AgentError::PhaseFailed { traces, .. } = err else {
            panic!("expected phase failure");
        };

        assert!(traces.iter().all(|trace| {
            trace.status == SeatRunStatus::Failed
                && trace.upstream_status == Some(429)
                && trace
                    .error
                    .as_deref()
                    .is_some_and(|error| error.contains("rate limited"))
        }));
    }

    #[tokio::test]
    async fn cancelled_session_does_not_continue() {
        let cancel = CancellationFlag::default();
        cancel.cancel();
        let err = runner(MockScenario::SuccessMajority)
            .run_session(Session::new("title", "topic", ""), cancel)
            .await
            .unwrap_err();
        assert!(matches!(err, AgentError::Cancelled));
    }

    #[tokio::test]
    async fn no_duplicate_phase_start_events() {
        let artifacts = runner(MockScenario::SuccessMajority)
            .run_session(
                Session::new("title", "topic", ""),
                CancellationFlag::default(),
            )
            .await
            .unwrap();
        assert!(!has_duplicate_phase_start(
            &artifacts.events,
            "independent_deliberation"
        ));
    }

    #[tokio::test]
    async fn mock_discussion_sets_idea_statuses() {
        let artifacts = runner(MockScenario::SuccessMajority)
            .run_session(
                Session::new("title", "topic", ""),
                CancellationFlag::default(),
            )
            .await
            .unwrap();
        let ideas = &artifacts.ideas;
        assert!(!ideas.is_empty());
        for idea in ideas {
            assert!(
                matches!(
                    idea.status,
                    IdeaStatus::Challenged
                        | IdeaStatus::Shortlisted
                        | IdeaStatus::Adopted
                        | IdeaStatus::Rejected
                ),
                "unexpected status {:?} for idea '{}'",
                idea.status,
                idea.title
            );
            if idea.status == IdeaStatus::Adopted {
                assert!(!idea.referenced_by_proposals.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn mock_discussion_publishes_evidence_pool() {
        let artifacts = runner(MockScenario::SuccessMajority)
            .run_session(
                Session::new("title", "topic", ""),
                CancellationFlag::default(),
            )
            .await
            .unwrap();
        assert!(
            !artifacts.claims.is_empty(),
            "claims should be extracted from ideas"
        );
        assert!(
            !artifacts.evidence.is_empty(),
            "evidence should be extracted"
        );
        for claim in &artifacts.claims {
            assert!(!claim.content.trim().is_empty());
            assert!(!claim.context.trim().is_empty());
        }
        for ev in &artifacts.evidence {
            assert!(!ev.content.trim().is_empty());
            assert!(matches!(
                ev.kind,
                wenyuan_core::EvidenceKind::Fact
                    | wenyuan_core::EvidenceKind::Inference
                    | wenyuan_core::EvidenceKind::Preference
            ));
        }
    }

    #[tokio::test]
    async fn smoke_test_five_complete_runs_with_phase4_data() {
        for index in 0..5 {
            let artifacts = runner(MockScenario::SuccessMajority)
                .run_session(
                    Session::new(format!("smoke {index}"), "topic", ""),
                    CancellationFlag::default(),
                )
                .await
                .unwrap();
            assert!(
                artifacts.decision.is_some(),
                "run {index}: missing decision"
            );
            assert!(!artifacts.ideas.is_empty(), "run {index}: no ideas");
            assert!(!artifacts.claims.is_empty(), "run {index}: no claims");
            assert!(!artifacts.evidence.is_empty(), "run {index}: no evidence");

            let adopted = artifacts
                .ideas
                .iter()
                .filter(|i| matches!(i.status, IdeaStatus::Adopted))
                .count();
            let shortlisted = artifacts
                .ideas
                .iter()
                .filter(|i| matches!(i.status, IdeaStatus::Shortlisted))
                .count();
            let challenged = artifacts
                .ideas
                .iter()
                .filter(|i| matches!(i.status, IdeaStatus::Challenged))
                .count();
            assert!(
                adopted + shortlisted + challenged == artifacts.ideas.len()
                    || artifacts
                        .ideas
                        .iter()
                        .any(|i| matches!(i.status, IdeaStatus::Rejected)),
                "run {index}: ideas should be in a valid lifecycle state (adopted={adopted}, shortlisted={shortlisted}, challenged={challenged}, total={})",
                artifacts.ideas.len()
            );

            let proposal_ids: Vec<Uuid> = artifacts.proposals.iter().map(|p| p.id).collect();
            for idea in &artifacts.ideas {
                if !idea.referenced_by_proposals.is_empty() {
                    for ref_id in &idea.referenced_by_proposals {
                        assert!(
                            proposal_ids.contains(ref_id),
                            "run {index}: idea references unknown proposal {ref_id}"
                        );
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn split_then_convergence_merges_with_minority_choices() {
        let artifacts = runner(MockScenario::SplitThenConvergence)
            .run_session(
                Session::new("title", "topic", ""),
                CancellationFlag::default(),
            )
            .await
            .unwrap();
        let decision = artifacts.decision.unwrap();
        assert_eq!(decision.status, DecisionStatus::NoMajority);
        assert!(decision.minority_choices.len() >= 2);
        let chizheng = decision
            .minority_choices
            .iter()
            .find(|m| m.seat == SeatKind::Chizheng)
            .expect("持正席 should be in minority");
        assert!(!chizheng.reassessment_condition.is_empty());
        assert!(chizheng.has_risk_warning);
        assert!(!decision.reassessment_conditions.is_empty());
        let has_merged = artifacts
            .proposals
            .iter()
            .any(|p| p.title.contains("合案") || p.source_idea_ids.len() >= 2);
        assert!(has_merged, "convergence should produce a merged proposal");
    }

    #[tokio::test]
    async fn smoke_test_five_runs_voting_strategy_fields() {
        for index in 0..5 {
            let artifacts = runner(MockScenario::SuccessMajority)
                .run_session(
                    Session::new(format!("phase5-smoke {index}"), "topic", ""),
                    CancellationFlag::default(),
                )
                .await
                .unwrap();
            let decision = artifacts.decision.unwrap();
            assert!(
                decision.minority_choices.is_empty(),
                "unanimous majority should have no minority in run {index}"
            );
            assert!(
                !decision.has_risk_blocker,
                "no risk blocker expected in run {index}"
            );
        }
    }

    #[tokio::test]
    async fn search_enabled_adds_evidence() {
        let mut session = Session::new("search test", "test topic for deliberation", "");
        session.search_enabled = true;
        let artifacts =
            AgentRunner::new(Arc::new(MockProvider::new(MockScenario::SuccessMajority)))
                .with_search(Arc::new(MockSearchBackend::new()))
                .run_session(session, CancellationFlag::default())
                .await
                .unwrap();
        assert!(
            artifacts
                .events
                .iter()
                .any(|e| e.starts_with("search_completed")),
            "expected search_completed event, got: {:?}",
            artifacts.events
        );
        let search_index = artifacts
            .events
            .iter()
            .position(|e| e.starts_with("search_completed"))
            .expect("search event should be present");
        let independent_index = artifacts
            .events
            .iter()
            .position(|e| e == "phase_started:independent_deliberation")
            .expect("independent phase should be present");
        assert!(
            search_index < independent_index,
            "search should run before independent deliberation: {:?}",
            artifacts.events
        );
        let evidence_sources: Vec<_> = artifacts
            .evidence
            .iter()
            .map(|e| e.source.clone())
            .collect();
        assert!(
            evidence_sources.iter().any(|s| s.contains("github.com")),
            "expected github source in evidence, got: {:?}",
            evidence_sources
        );
        assert!(
            artifacts
                .tool_runs
                .iter()
                .any(|run| run.tool_name == "web_search" && run.status == "completed"),
            "expected web_search tool run, got: {:?}",
            artifacts.tool_runs
        );
    }

    #[tokio::test]
    async fn external_evidence_is_preserved_in_artifacts() {
        let mut session = Session::new("file evidence test", "topic", "");
        session.external_evidence.push(Evidence {
            id: Uuid::new_v4(),
            proposed_by: SeatKind::Mouyuan,
            kind: wenyuan_core::EvidenceKind::Fact,
            content: "DOCX source fact".into(),
            source: "file://source.docx#document:0".into(),
            source_fetched_at: Some(chrono::Utc::now().to_rfc3339()),
            source_hash: Some("hash".into()),
            claim_ids: vec![],
            source_kind: EvidenceSourceKind::File,
            trust_level: EvidenceTrustLevel::UntrustedExternal,
            safety_flags: SourceSafetyFlags::default(),
        });
        session.external_tool_runs.push(ToolRun {
            id: Uuid::new_v4(),
            seat: None,
            phase: None,
            tool_name: "document_parse".into(),
            input_summary: "filename=source.docx".into(),
            input_hash: "hash".into(),
            status: "completed".into(),
            duration_ms: 3,
            evidence_ids: vec![],
            error: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        });

        let artifacts = runner(MockScenario::SuccessMajority)
            .run_session(session, CancellationFlag::default())
            .await
            .unwrap();

        assert!(
            artifacts.evidence.iter().any(|ev| {
                ev.source == "file://source.docx#document:0"
                    && ev.source_kind == EvidenceSourceKind::File
            }),
            "expected file evidence to be preserved, got: {:?}",
            artifacts.evidence
        );
        assert!(
            artifacts
                .tool_runs
                .iter()
                .any(|run| run.tool_name == "document_parse"),
            "expected document_parse tool run, got: {:?}",
            artifacts.tool_runs
        );
    }

    #[tokio::test]
    async fn scribe_enabled_produces_report() {
        let mut session = Session::new("scribe test", "topic", "");
        session.scribe_enabled = true;
        let artifacts = runner(MockScenario::SuccessMajority)
            .run_session(session, CancellationFlag::default())
            .await
            .unwrap();
        let report = artifacts
            .scribe_report
            .expect("scribe should produce a report");
        assert!(!report.consensus_summary.is_empty());
        assert!(!report.final_report.is_empty());
        assert!(artifacts.events.iter().any(|e| e == "scribe_completed"));
    }

    struct CapturingSearchBackend {
        mock: MockSearchBackend,
        captured: Arc<std::sync::Mutex<Option<String>>>,
    }

    #[async_trait::async_trait]
    impl SearchBackend for CapturingSearchBackend {
        fn name(&self) -> &'static str {
            "capturing"
        }

        async fn search(
            &self,
            query: &str,
            limit: usize,
        ) -> Result<Vec<SearchResult>, SearchError> {
            {
                let mut guard = self.captured.lock().unwrap();
                *guard = Some(query.to_string());
            }
            self.mock.search(query, limit).await
        }
    }

    #[tokio::test]
    async fn search_passes_full_cjk_topic_as_query() {
        let long_topic = "开缸半年了，一些sps和lps，60*45*45cm的背滤缸，3条小丑鱼（3cm），1条5cm蓝吊，1条4cm三角吊，一条5cm狐狸鱼，10只+的螺。蛋分/造浪都齐全，日出日落全光谱的led珊瑚灯（max100w），每2周换10%水，现在活石上没什么藻，但正面的玻璃上基本一周多就有绿藻长满，是顽固的那种要刮藻刀才刮的下来，是否水质不稳定？";

        let captured = Arc::new(std::sync::Mutex::new(None));
        let search_backend = CapturingSearchBackend {
            mock: MockSearchBackend::new(),
            captured: captured.clone(),
        };

        let mut session = Session::new("CJK search test", long_topic, "");
        session.search_enabled = true;

        AgentRunner::new(Arc::new(MockProvider::new(MockScenario::SuccessMajority)))
            .with_search(Arc::new(search_backend))
            .run_session(session, CancellationFlag::default())
            .await
            .unwrap();

        let captured_query = captured.lock().unwrap().take();
        assert!(captured_query.is_some(), "search backend was never called");
        let query = captured_query.unwrap();
        // The mock returns {"query":"鱼缸 水质 褐藻"} for search-keywords-v1.
        // After clean_search_query, the query should be "鱼缸 水质 褐藻".
        assert!(
            query.len() < long_topic.len(),
            "extracted query should be shorter than full topic.\nTopic ({} bytes): {}\nQuery ({} bytes): {}",
            long_topic.len(),
            long_topic,
            query.len(),
            query
        );
        assert!(
            query.contains("鱼缸"),
            "query should contain extracted keywords, got: {query}"
        );
    }

    #[tokio::test]
    async fn search_query_extracts_first_sentence_from_very_long_topic() {
        // Build a topic well over 200 Unicode chars to ensure keyword extraction is required
        let long_base = "开缸半年了，一些sps和lps，60*45*45cm的背滤缸。";
        let padding: String = std::iter::repeat("这是一个非常长的重复填充文本，目的是让整个话题超过两百个Unicode字符的限制以确保搜索查询被截断。").take(5).collect();
        let very_long = format!("{long_base}{padding}");
        assert!(
            very_long.chars().count() > 200,
            "test topic must exceed 200 chars"
        );

        let captured = Arc::new(std::sync::Mutex::new(None));
        let search_backend = CapturingSearchBackend {
            mock: MockSearchBackend::new(),
            captured: captured.clone(),
        };

        let mut session = Session::new("very long topic", &very_long, "");
        session.search_enabled = true;

        AgentRunner::new(Arc::new(MockProvider::new(MockScenario::SuccessMajority)))
            .with_search(Arc::new(search_backend))
            .run_session(session, CancellationFlag::default())
            .await
            .unwrap();

        let captured_query = captured.lock().unwrap().take();
        assert!(captured_query.is_some(), "search should have been called");
        let query = captured_query.unwrap();
        // Mock returns {"query":"鱼缸 水质 褐藻"} for search-keywords-v1.
        // Verify extracted query is shorter than full topic and contains keywords.
        assert!(
            query.len() < very_long.len(),
            "extracted query should be shorter than full topic: query={} bytes vs topic={} bytes",
            query.len(),
            very_long.len()
        );
        assert!(
            query.contains("鱼缸"),
            "query should contain extracted keywords, got: {query}"
        );
    }

    #[test]
    fn clean_search_query_parses_json() {
        let result = clean_search_query(r#"{"query":"鱼缸 水质 褐藻"}"#);
        assert_eq!(result, "鱼缸 水质 褐藻");
    }

    #[test]
    fn clean_search_query_parses_json_with_spaces() {
        let result = clean_search_query(r#"{"query": "鱼缸 水质 褐藻"}"#);
        assert_eq!(result, "鱼缸 水质 褐藻");
    }

    #[test]
    fn clean_search_query_strips_prefix() {
        let result = clean_search_query("关键词：鱼缸 水质");
        assert_eq!(result, "鱼缸 水质");
    }

    #[test]
    fn clean_search_query_strips_markdown_fence() {
        let result = clean_search_query("```json\n{\"query\": \"鱼缸 水质\"}\n```");
        assert_eq!(result, "鱼缸 水质");
    }

    #[test]
    fn clean_search_query_takes_first_line() {
        let result = clean_search_query("鱼缸 水质\n褐藻 造浪");
        assert_eq!(result, "鱼缸 水质");
    }

    #[test]
    fn clean_search_query_fallback_to_plain_text() {
        let result = clean_search_query("鱼缸 水质 褐藻");
        assert_eq!(result, "鱼缸 水质 褐藻");
    }

    #[test]
    fn valid_search_query_rejects_single_chinese_char() {
        assert!(!valid_search_query("鱼"));
    }

    #[test]
    fn valid_search_query_rejects_two_chinese_chars() {
        assert!(!valid_search_query("鱼缸"));
    }

    #[test]
    fn valid_search_query_accepts_three_chinese_chars() {
        assert!(valid_search_query("鱼缸水"));
    }

    #[test]
    fn valid_search_query_accepts_chinese_with_spaces() {
        assert!(valid_search_query("鱼缸 水质"));
    }

    #[test]
    fn valid_search_query_rejects_empty() {
        assert!(!valid_search_query(""));
    }

    #[test]
    fn valid_search_query_rejects_whitespace_only() {
        assert!(!valid_search_query("   "));
    }
}
