use async_trait::async_trait;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Notify;
use uuid::Uuid;
use wenyuan_core::{
    ChatMessage, Critique, Decision, IdeaCard, Proposal, SeatKind, SeatStatus, Session,
    SessionPhase, Vote, VoteOutcome, VotePolicy, build_decision, phase_barrier_completed,
};
use wenyuan_provider::{LlmProvider, LlmRequest, LlmResponse, ProviderError};

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
    #[error("phase {phase:?} failed with {failures} failed seat run(s)")]
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
}

impl AgentRunner {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            provider,
            timeout: Duration::from_millis(250),
            policy: VotePolicy::default(),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn run_session(
        &self,
        session: Session,
        cancel: CancellationFlag,
    ) -> Result<DiscussionArtifacts, AgentError> {
        self.run_session_with_progress(session, cancel, None).await
    }

    pub async fn run_session_with_progress(
        &self,
        mut session: Session,
        cancel: CancellationFlag,
        progress: Option<Arc<dyn ProgressSink>>,
    ) -> Result<DiscussionArtifacts, AgentError> {
        let mut artifacts = DiscussionArtifacts::default();

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
        let (ideas, traces) = self
            .run_independent(&session, &cancel, progress.as_ref())
            .await?;
        artifacts.ideas = ideas;
        artifacts.seat_runs.extend(traces);
        artifacts
            .events
            .push("phase_completed:independent_deliberation".into());
        emit_progress(
            progress.as_ref(),
            "phase_completed",
            serde_json::json!({ "phase": SessionPhase::IndependentDeliberation }),
        )
        .await;

        self.ensure_not_cancelled(&cancel)?;
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
        let (critiques, traces) = self
            .run_critiques(session.id, &artifacts.ideas, &cancel, progress.as_ref())
            .await?;
        artifacts.critiques = critiques;
        artifacts.seat_runs.extend(traces);
        artifacts
            .events
            .push("phase_completed:cross_critique".into());
        emit_progress(
            progress.as_ref(),
            "phase_completed",
            serde_json::json!({ "phase": SessionPhase::CrossCritique }),
        )
        .await;

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
        let (proposals, traces) = self
            .run_revision(
                session.id,
                &artifacts.ideas,
                &artifacts.critiques,
                &cancel,
                progress.as_ref(),
            )
            .await?;
        artifacts.proposals = proposals;
        artifacts.seat_runs.extend(traces);
        artifacts.events.push("phase_completed:revision".into());
        emit_progress(
            progress.as_ref(),
            "phase_completed",
            serde_json::json!({ "phase": SessionPhase::Revision }),
        )
        .await;

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
            .run_voting(session.id, &artifacts.proposals, &cancel, progress.as_ref())
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

        match wenyuan_core::tally_votes(
            &artifacts.proposals,
            &decision_votes,
            &self.policy,
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
                let (mut second_votes, traces) = self
                    .run_voting(session.id, &artifacts.proposals, &cancel, progress.as_ref())
                    .await?;
                artifacts.seat_runs.extend(traces);
                decision_votes = second_votes.clone();
                artifacts.votes.append(&mut second_votes);
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
            &self.policy,
            session.convergence_used,
        )
        .map_err(|err| AgentError::Core(err.to_string()))?;
        session.result = Some(decision.clone());
        artifacts.decision = Some(decision);
        artifacts.quality = DiscussionQualityMetrics::calculate(&artifacts);
        artifacts.session = Some(session);
        artifacts.events.push("session_completed".into());
        Ok(artifacts)
    }

    async fn run_independent(
        &self,
        session: &Session,
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
    ) -> Result<(Vec<IdeaCard>, Vec<SeatRunTrace>), AgentError> {
        let input = serde_json::json!({
            "title": session.title,
            "topic": session.topic,
            "context": session.context,
            "rule": "第一轮独议完全隔离，不引用其他席位输出。"
        });
        let run = self
            .run_three::<IndependentOutput>(
                session.id,
                SessionPhase::IndependentDeliberation,
                input,
                cancel,
                progress,
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
                });
            }
        }
        Ok((ideas, run.traces))
    }

    async fn run_critiques(
        &self,
        session_id: Uuid,
        ideas: &[IdeaCard],
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
    ) -> Result<(Vec<Critique>, Vec<SeatRunTrace>), AgentError> {
        let input = serde_json::json!({
            "ideas": ideas,
            "rule": "只批议其他席位的独议结果，必须逐席给出结构化批议。"
        });
        let run = self
            .run_three::<CritiqueOutput>(
                session_id,
                SessionPhase::CrossCritique,
                input,
                cancel,
                progress,
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
        ))
    }

    async fn run_revision(
        &self,
        session_id: Uuid,
        ideas: &[IdeaCard],
        critiques: &[Critique],
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
    ) -> Result<(Vec<Proposal>, Vec<SeatRunTrace>), AgentError> {
        let input = serde_json::json!({
            "ideas": ideas,
            "critiques": critiques,
            "rule": "形成正式策案，说明采纳、拒绝、相较独议修改和置信度。"
        });
        let run = self
            .run_three::<ProposalOutput>(
                session_id,
                SessionPhase::Revision,
                input,
                cancel,
                progress,
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
        ))
    }

    async fn run_voting(
        &self,
        session_id: Uuid,
        proposals: &[Proposal],
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
    ) -> Result<(Vec<Vote>, Vec<SeatRunTrace>), AgentError> {
        let input = serde_json::json!({
            "proposals": proposals,
            "rule": "对三个策案匿名投票，proposal_ref 使用 proposal_0、proposal_1、proposal_2。"
        });
        let run = self
            .run_three::<VoteOutput>(session_id, SessionPhase::Voting, input, cancel, progress)
            .await?;
        let mut votes = Vec::new();
        for (voter, output) in run.outputs {
            for raw in output.votes {
                let index = raw
                    .proposal_ref
                    .trim_start_matches("proposal_")
                    .parse::<usize>()
                    .unwrap_or(0);
                let Some(proposal) = proposals.get(index) else {
                    continue;
                };
                votes.push(Vote {
                    voter,
                    proposal_id: proposal.id,
                    value_score: raw.value_score,
                    novelty_score: raw.novelty_score,
                    feasibility_score: raw.feasibility_score,
                    risk_score: raw.risk_score,
                    roi_score: raw.roi_score,
                    final_choice: raw.final_choice,
                    reason: raw.reason,
                    confidence: raw.confidence,
                });
            }
        }
        Ok((votes, run.traces))
    }

    async fn run_three<T>(
        &self,
        session_id: Uuid,
        phase: SessionPhase,
        phase_input: serde_json::Value,
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
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
            )
        });
        let results = join_all(futures).await;
        let mut outputs = Vec::new();
        let mut traces = Vec::new();
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
        }

        if cancelled {
            return Err(AgentError::Cancelled);
        }

        if let Err(err) = phase_barrier_completed(&statuses) {
            let failures = traces
                .iter()
                .filter(|trace| trace.status == SeatRunStatus::Failed)
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

        Ok(PhaseRun { outputs, traces })
    }

    async fn call_and_parse<T>(
        &self,
        session_id: Uuid,
        seat: SeatKind,
        phase: SessionPhase,
        phase_input: serde_json::Value,
        cancel: &CancellationFlag,
        progress: Option<&Arc<dyn ProgressSink>>,
    ) -> Result<SeatCallResult<T>, AgentError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut traces = Vec::new();
        let prompt = prompt_config(seat);
        emit_progress(
            progress,
            "seat_started",
            serde_json::json!({ "seat": seat, "phase": phase }),
        )
        .await;
        let request = self.build_request(session_id, seat, phase, false, &prompt, &phase_input);
        let (first, first_duration_ms) = self.call_provider(request, cancel).await;
        let Ok(response) = first else {
            if matches!(first, Err(ProviderError::Cancelled)) {
                return Err(AgentError::Cancelled);
            }
            let error = first.err().unwrap_or_else(unknown_provider_error);
            emit_progress(
                progress,
                "seat_failed",
                serde_json::json!({ "seat": seat, "phase": phase, "error": error.to_string() }),
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
            });
        };

        match serde_json::from_str::<T>(&response.content) {
            Ok(parsed) => {
                emit_progress(
                    progress,
                    "seat_completed",
                    serde_json::json!({ "seat": seat, "phase": phase }),
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
                })
            }
            Err(err) => {
                traces.push(SeatRunTrace::failed_parse(
                    TraceContext::new(session_id, seat, phase, &prompt, false, first_duration_ms),
                    &response,
                    err.to_string(),
                ));
                let repair_request =
                    self.build_request(session_id, seat, phase, true, &prompt, &phase_input);
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
                        serde_json::json!({ "seat": seat, "phase": phase, "error": error.to_string() }),
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
                    });
                };
                match serde_json::from_str::<T>(&repaired_response.content) {
                    Ok(parsed) => {
                        emit_progress(
                            progress,
                            "seat_completed",
                            serde_json::json!({ "seat": seat, "phase": phase, "repair_attempted": true }),
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
                        })
                    }
                    Err(err) => {
                        emit_progress(
                            progress,
                            "seat_failed",
                            serde_json::json!({ "seat": seat, "phase": phase, "error": err.to_string(), "repair_attempted": true }),
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
                        })
                    }
                }
            }
        }
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

    fn build_request(
        &self,
        session_id: Uuid,
        seat: SeatKind,
        phase: SessionPhase,
        repair_json: bool,
        prompt: &SeatPrompt,
        phase_input: &serde_json::Value,
    ) -> LlmRequest {
        let instruction = if repair_json {
            "上一轮输出不是合法 JSON。请仅根据同一阶段输入修复格式，并只返回合法 JSON。"
        } else {
            "请执行当前阶段并只返回合法 JSON。"
        };
        LlmRequest {
            session_id,
            seat,
            phase,
            repair_json,
            temperature: prompt.temperature,
            max_tokens: prompt.max_tokens,
            prompt_version: prompt.version.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: prompt.content.to_string(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: format!(
                        "{instruction}\n阶段：{phase:?}\n输入：{}",
                        serde_json::to_string(phase_input).unwrap_or_else(|_| "{}".into())
                    ),
                },
            ],
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
    pub events: Vec<String>,
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
    fn calculate(artifacts: &DiscussionArtifacts) -> Self {
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
                            && !critique.hidden_assumption.trim().is_empty()
                            && !critique.counterexample.trim().is_empty()
                            && !critique.suggested_improvement.trim().is_empty()
                            && !critique.evidence_question.trim().is_empty()
                    })
                    .count(),
                artifacts.critiques.len(),
            ),
            revision_change_rate: ratio(
                artifacts
                    .proposals
                    .iter()
                    .filter(|proposal| !proposal.changes_from_initial.is_empty())
                    .count(),
                artifacts.proposals.len(),
            ),
            self_vote_rate: artifacts
                .decision
                .as_ref()
                .map(|decision| ratio(decision.self_vote_count, artifacts.votes.len()))
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
    temperature: f32,
    max_tokens: u32,
}

pub fn system_prompt(seat: SeatKind) -> &'static str {
    prompt_config(seat).content
}

fn prompt_config(seat: SeatKind) -> SeatPrompt {
    match seat {
        SeatKind::Mouyuan => SeatPrompt {
            version: "mouyuan-v1",
            content: include_str!("../prompts/mouyuan-v1.md"),
            temperature: 0.7,
            max_tokens: 900,
        },
        SeatKind::Jingshi => SeatPrompt {
            version: "jingshi-v1",
            content: include_str!("../prompts/jingshi-v1.md"),
            temperature: 0.4,
            max_tokens: 800,
        },
        SeatKind::Chizheng => SeatPrompt {
            version: "chizheng-v1",
            content: include_str!("../prompts/chizheng-v1.md"),
            temperature: 0.2,
            max_tokens: 800,
        },
    }
}

struct PhaseRun<T> {
    outputs: Vec<(SeatKind, T)>,
    traces: Vec<SeatRunTrace>,
}

struct SeatCallResult<T> {
    seat: SeatKind,
    parsed: Option<T>,
    traces: Vec<SeatRunTrace>,
}

fn unknown_provider_error() -> ProviderError {
    ProviderError::Request("unknown provider error".to_string())
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

fn token_set(value: &str) -> HashSet<String> {
    value
        .split(|ch: char| ch.is_whitespace() || ch.is_ascii_punctuation())
        .map(normalize_for_metric)
        .filter(|token| !token.is_empty())
        .collect()
}

#[derive(Debug, Deserialize)]
struct IndependentOutput {
    ideas: Vec<RawIdea>,
}

#[derive(Debug, Deserialize)]
struct RawIdea {
    title: String,
    summary: String,
    value: String,
    mechanism: String,
    #[serde(default)]
    unconventional: bool,
    assumptions: Vec<String>,
    risks: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CritiqueOutput {
    reviews: Vec<RawCritique>,
}

#[derive(Debug, Deserialize)]
struct RawCritique {
    target_seat: SeatKind,
    strongest_point: String,
    weakest_point: String,
    hidden_assumption: String,
    challenge: String,
    #[serde(default)]
    counterexample: String,
    suggested_improvement: String,
    #[serde(default)]
    evidence_question: String,
}

#[derive(Debug, Deserialize)]
struct ProposalOutput {
    title: String,
    summary: String,
    #[serde(default)]
    source_idea_ids: Vec<Uuid>,
    #[serde(default)]
    adopted_points: Vec<String>,
    #[serde(default)]
    rejected_points: Vec<String>,
    #[serde(default)]
    rejection_reasons: Vec<String>,
    #[serde(default)]
    changes_from_initial: Vec<String>,
    user_value: String,
    implementation_path: String,
    risks: Vec<String>,
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
    proposal_ref: String,
    value_score: u8,
    novelty_score: u8,
    feasibility_score: u8,
    risk_score: u8,
    roi_score: u8,
    final_choice: bool,
    reason: String,
    confidence: f32,
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
        assert_eq!(prompts[0].version, "mouyuan-v1");
        assert_eq!(prompts[1].temperature, 0.4);
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
}
