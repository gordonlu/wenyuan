use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;
use wenyuan_core::{
    ChatMessage, Critique, Decision, IdeaCard, Proposal, SeatKind, SeatStatus, Session,
    SessionPhase, Vote, VoteOutcome, VotePolicy, build_decision, phase_barrier_completed,
};
use wenyuan_provider::{LlmProvider, LlmRequest, LlmResponse, ProviderError};

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

#[derive(Debug, Default, Clone)]
pub struct CancellationFlag(Arc<AtomicBool>);

impl CancellationFlag {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
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
        mut session: Session,
        cancel: CancellationFlag,
    ) -> Result<DiscussionArtifacts, AgentError> {
        let mut artifacts = DiscussionArtifacts::default();

        self.ensure_not_cancelled(&cancel)?;
        session
            .transition_to(SessionPhase::IndependentDeliberation)
            .map_err(|err| AgentError::Core(err.to_string()))?;
        artifacts
            .events
            .push("phase_started:independent_deliberation".into());
        let (ideas, traces) = self.run_independent(session.id, &cancel).await?;
        artifacts.ideas = ideas;
        artifacts.seat_runs.extend(traces);
        artifacts
            .events
            .push("phase_completed:independent_deliberation".into());

        self.ensure_not_cancelled(&cancel)?;
        session
            .transition_to(SessionPhase::CrossCritique)
            .map_err(|err| AgentError::Core(err.to_string()))?;
        artifacts.events.push("phase_started:cross_critique".into());
        let (critiques, traces) = self.run_critiques(session.id, &cancel).await?;
        artifacts.critiques = critiques;
        artifacts.seat_runs.extend(traces);
        artifacts
            .events
            .push("phase_completed:cross_critique".into());

        self.ensure_not_cancelled(&cancel)?;
        session
            .transition_to(SessionPhase::Revision)
            .map_err(|err| AgentError::Core(err.to_string()))?;
        artifacts.events.push("phase_started:revision".into());
        let (proposals, traces) = self.run_revision(session.id, &cancel).await?;
        artifacts.proposals = proposals;
        artifacts.seat_runs.extend(traces);
        artifacts.events.push("phase_completed:revision".into());

        self.ensure_not_cancelled(&cancel)?;
        session
            .transition_to(SessionPhase::Voting)
            .map_err(|err| AgentError::Core(err.to_string()))?;
        artifacts.events.push("phase_started:voting".into());
        let (votes, traces) = self
            .run_voting(session.id, &artifacts.proposals, &cancel)
            .await?;
        artifacts.votes = votes;
        artifacts.seat_runs.extend(traces);
        artifacts.events.push("phase_completed:voting".into());
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
                let (mut second_votes, traces) = self
                    .run_voting(session.id, &artifacts.proposals, &cancel)
                    .await?;
                artifacts.seat_runs.extend(traces);
                decision_votes = second_votes.clone();
                artifacts.votes.append(&mut second_votes);
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
        artifacts.session = Some(session);
        artifacts.events.push("session_completed".into());
        Ok(artifacts)
    }

    async fn run_independent(
        &self,
        session_id: Uuid,
        cancel: &CancellationFlag,
    ) -> Result<(Vec<IdeaCard>, Vec<SeatRunTrace>), AgentError> {
        let run = self
            .run_three::<IndependentOutput>(
                session_id,
                SessionPhase::IndependentDeliberation,
                cancel,
            )
            .await?;
        let mut ideas = Vec::new();
        for (seat, output) in run.outputs {
            for idea in output.ideas.into_iter().take(5) {
                ideas.push(IdeaCard {
                    id: Uuid::new_v4(),
                    proposed_by: seat,
                    title: idea.title,
                    summary: idea.summary,
                    value: idea.value,
                    mechanism: idea.mechanism,
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
        cancel: &CancellationFlag,
    ) -> Result<(Vec<Critique>, Vec<SeatRunTrace>), AgentError> {
        let run = self
            .run_three::<CritiqueOutput>(session_id, SessionPhase::CrossCritique, cancel)
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
                        suggested_improvement: review.suggested_improvement,
                    })
                })
                .collect(),
            run.traces,
        ))
    }

    async fn run_revision(
        &self,
        session_id: Uuid,
        cancel: &CancellationFlag,
    ) -> Result<(Vec<Proposal>, Vec<SeatRunTrace>), AgentError> {
        let run = self
            .run_three::<ProposalOutput>(session_id, SessionPhase::Revision, cancel)
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
                    user_value: output.user_value,
                    implementation_path: output.implementation_path,
                    risks: output.risks,
                    success_metrics: output.success_metrics,
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
    ) -> Result<(Vec<Vote>, Vec<SeatRunTrace>), AgentError> {
        let run = self
            .run_three::<VoteOutput>(session_id, SessionPhase::Voting, cancel)
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
        cancel: &CancellationFlag,
    ) -> Result<PhaseRun<T>, AgentError>
    where
        T: for<'de> Deserialize<'de> + Send + 'static,
    {
        self.ensure_not_cancelled(cancel)?;
        let futures = SeatKind::ALL
            .into_iter()
            .map(|seat| self.call_and_parse::<T>(session_id, seat, phase));
        let results = join_all(futures).await;
        let mut outputs = Vec::new();
        let mut traces = Vec::new();
        let mut statuses = Vec::new();

        for result in results {
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
    ) -> SeatCallResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut traces = Vec::new();
        let prompt = prompt_config(seat);
        let request = self.build_request(session_id, seat, phase, false, &prompt);
        let (first, first_duration_ms) = self.call_provider(seat, request).await;
        let Ok(response) = first else {
            traces.push(SeatRunTrace::failed_provider(
                TraceContext::new(session_id, seat, phase, &prompt, false, first_duration_ms),
                response_error(first.err()),
            ));
            return SeatCallResult {
                seat,
                parsed: None,
                traces,
            };
        };

        match serde_json::from_str::<T>(&response.content) {
            Ok(parsed) => {
                traces.push(SeatRunTrace::completed(
                    TraceContext::new(session_id, seat, phase, &prompt, false, first_duration_ms),
                    &response,
                ));
                SeatCallResult {
                    seat,
                    parsed: Some(parsed),
                    traces,
                }
            }
            Err(err) => {
                traces.push(SeatRunTrace::failed_parse(
                    TraceContext::new(session_id, seat, phase, &prompt, false, first_duration_ms),
                    &response,
                    err.to_string(),
                ));
                let repair_request = self.build_request(session_id, seat, phase, true, &prompt);
                let (repaired, repair_duration_ms) = self.call_provider(seat, repair_request).await;
                let Ok(repaired_response) = repaired else {
                    traces.push(SeatRunTrace::failed_provider(
                        TraceContext::new(
                            session_id,
                            seat,
                            phase,
                            &prompt,
                            true,
                            repair_duration_ms,
                        ),
                        response_error(repaired.err()),
                    ));
                    return SeatCallResult {
                        seat,
                        parsed: None,
                        traces,
                    };
                };
                match serde_json::from_str::<T>(&repaired_response.content) {
                    Ok(parsed) => {
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
                        SeatCallResult {
                            seat,
                            parsed: Some(parsed),
                            traces,
                        }
                    }
                    Err(err) => {
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
                        SeatCallResult {
                            seat,
                            parsed: None,
                            traces,
                        }
                    }
                }
            }
        }
    }

    async fn call_provider(
        &self,
        seat: SeatKind,
        request: LlmRequest,
    ) -> (Result<LlmResponse, ProviderError>, u128) {
        let started = Instant::now();
        let response = tokio::time::timeout(self.timeout, self.provider.complete(request)).await;
        let duration_ms = started.elapsed().as_millis();
        match response {
            Ok(result) => (result, duration_ms),
            Err(_) => {
                let _ = seat;
                (Err(ProviderError::Timeout), duration_ms)
            }
        }
    }

    fn build_request(
        &self,
        session_id: Uuid,
        seat: SeatKind,
        phase: SessionPhase,
        repair_json: bool,
        prompt: &SeatPrompt,
    ) -> LlmRequest {
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
                    content: format!("请执行 {phase:?} 阶段并只返回 JSON。"),
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
    pub events: Vec<String>,
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

    fn failed_provider(ctx: TraceContext<'_>, error: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: ctx.session_id,
            seat: ctx.seat,
            phase: ctx.phase,
            status: SeatRunStatus::Failed,
            prompt_version: ctx.prompt.version.to_string(),
            repair_attempted: ctx.repair_attempted,
            raw_output: None,
            error: Some(error),
            duration_ms: ctx.duration_ms,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            upstream_status: None,
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

fn response_error(error: Option<ProviderError>) -> String {
    error
        .map(|error| error.to_string())
        .unwrap_or_else(|| "unknown provider error".to_string())
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
    suggested_improvement: String,
}

#[derive(Debug, Deserialize)]
struct ProposalOutput {
    title: String,
    summary: String,
    source_idea_ids: Vec<Uuid>,
    user_value: String,
    implementation_path: String,
    risks: Vec<String>,
    success_metrics: Vec<String>,
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
    use wenyuan_core::DecisionStatus;
    use wenyuan_provider::{MockProvider, MockScenario};

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
        assert_eq!(artifacts.ideas.len(), 3);
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
