use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeatKind {
    Mouyuan,
    Jingshi,
    Chizheng,
}

impl SeatKind {
    pub const ALL: [SeatKind; 3] = [SeatKind::Mouyuan, SeatKind::Jingshi, SeatKind::Chizheng];
    pub const SINGLE: [SeatKind; 1] = [SeatKind::Mouyuan];

    pub fn label(self) -> &'static str {
        match self {
            SeatKind::Mouyuan => "谋远席",
            SeatKind::Jingshi => "经世席",
            SeatKind::Chizheng => "持正席",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliberationMode {
    #[default]
    ThreeSeat,
    SingleAgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
    Draft,
    IndependentDeliberation,
    CrossCritique,
    Revision,
    Voting,
    Convergence,
    Completed,
    Failed,
    Cancelled,
}

impl SessionPhase {
    pub fn can_transition_to(self, next: SessionPhase) -> bool {
        use SessionPhase::*;
        matches!(
            (self, next),
            (Draft, IndependentDeliberation)
                | (IndependentDeliberation, CrossCritique)
                | (CrossCritique, Revision)
                | (Revision, Voting)
                | (Voting, Convergence)
                | (Voting, Completed)
                | (Revision, Completed)
                | (Convergence, Voting)
                | (Convergence, Completed)
                | (_, Failed)
                | (_, Cancelled)
        ) && !matches!(self, Completed | Failed | Cancelled)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("illegal phase transition from {from:?} to {to:?}")]
    IllegalPhaseTransition {
        from: SessionPhase,
        to: SessionPhase,
    },
    #[error("phase barrier not satisfied")]
    PhaseBarrierNotSatisfied,
    #[error("convergence already used")]
    ConvergenceAlreadyUsed,
    #[error("vote references unknown proposal {0}")]
    UnknownProposal(Uuid),
    #[error("{seat:?} vote is incomplete: {reason}")]
    InvalidVote { seat: SeatKind, reason: String },
    #[error("{seat:?} raised a blocking issue: {issue}")]
    RiskVetoed { seat: SeatKind, issue: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeatStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatModelConfig {
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub title: String,
    pub topic: String,
    pub context: String,
    pub mode: DeliberationMode,
    pub phase: SessionPhase,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub result: Option<Decision>,
    pub failure_reason: Option<String>,
    pub convergence_used: bool,
    pub model_config: Option<HashMap<SeatKind, SeatModelConfig>>,
    pub vote_policy: Option<VotePolicy>,
    pub scribe_enabled: bool,
    pub search_enabled: bool,
    #[serde(default)]
    pub external_evidence: Vec<Evidence>,
    #[serde(default)]
    pub external_tool_runs: Vec<ToolRun>,
}

impl Session {
    pub fn new(
        title: impl Into<String>,
        topic: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            topic: topic.into(),
            context: context.into(),
            mode: DeliberationMode::default(),
            phase: SessionPhase::Draft,
            created_at: now,
            updated_at: now,
            result: None,
            failure_reason: None,
            convergence_used: false,
            model_config: None,
            vote_policy: None,
            scribe_enabled: false,
            search_enabled: false,
            external_evidence: vec![],
            external_tool_runs: vec![],
        }
    }

    pub fn new_with_mode(
        title: impl Into<String>,
        topic: impl Into<String>,
        context: impl Into<String>,
        mode: DeliberationMode,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            topic: topic.into(),
            context: context.into(),
            mode,
            phase: SessionPhase::Draft,
            created_at: now,
            updated_at: now,
            result: None,
            failure_reason: None,
            convergence_used: false,
            model_config: None,
            vote_policy: None,
            scribe_enabled: false,
            search_enabled: false,
            external_evidence: vec![],
            external_tool_runs: vec![],
        }
    }

    pub fn transition_to(&mut self, next: SessionPhase) -> Result<(), CoreError> {
        if !self.phase.can_transition_to(next) {
            return Err(CoreError::IllegalPhaseTransition {
                from: self.phase,
                to: next,
            });
        }
        if next == SessionPhase::Convergence && self.convergence_used {
            return Err(CoreError::ConvergenceAlreadyUsed);
        }
        if next == SessionPhase::Convergence {
            self.convergence_used = true;
        }
        self.phase = next;
        self.updated_at = Utc::now();
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seat {
    pub kind: SeatKind,
    pub system_prompt: String,
    pub conversation: Vec<ChatMessage>,
    pub provider_ref: String,
    pub status: SeatStatus,
    pub step_budget: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdeaStatus {
    #[default]
    Proposed,
    Expanded,
    Challenged,
    Merged,
    Shortlisted,
    Rejected,
    Adopted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaCard {
    pub id: Uuid,
    pub proposed_by: SeatKind,
    #[serde(default)]
    pub source_seats: Vec<SeatKind>,
    pub title: String,
    pub summary: String,
    pub value: String,
    pub mechanism: String,
    #[serde(default)]
    pub unconventional: bool,
    pub assumptions: Vec<String>,
    pub risks: Vec<String>,
    #[serde(default)]
    pub status: IdeaStatus,
    #[serde(default)]
    pub challenged_by: Vec<Uuid>,
    #[serde(default)]
    pub referenced_by_proposals: Vec<Uuid>,
    #[serde(default)]
    pub merged_into: Option<Uuid>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    #[default]
    Fact,
    Inference,
    Preference,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    #[default]
    Proposed,
    Verified,
    Disputed,
    Rejected,
    Superseded,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSourceKind {
    #[default]
    Internal,
    WebSearch,
    File,
    Code,
    Log,
    Data,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceTrustLevel {
    #[default]
    Internal,
    UntrustedExternal,
    UserProvided,
    VerifiedExternal,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSafetyFlags {
    #[serde(default)]
    pub prompt_injection_risk: bool,
    #[serde(default)]
    pub contains_control_chars: bool,
    #[serde(default)]
    pub truncated: bool,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: Uuid,
    pub proposed_by: SeatKind,
    pub content: String,
    pub context: String,
    #[serde(default)]
    pub is_supported: bool,
    #[serde(default)]
    pub evidence_ids: Vec<Uuid>,
    #[serde(default)]
    pub assessment_ids: Vec<Uuid>,
    #[serde(default)]
    pub status: EvidenceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: Uuid,
    pub proposed_by: SeatKind,
    pub kind: EvidenceKind,
    pub content: String,
    pub source: String,
    #[serde(default)]
    pub source_fetched_at: Option<String>,
    #[serde(default)]
    pub source_hash: Option<String>,
    #[serde(default)]
    pub claim_ids: Vec<Uuid>,
    #[serde(default)]
    pub source_kind: EvidenceSourceKind,
    #[serde(default)]
    pub trust_level: EvidenceTrustLevel,
    #[serde(default)]
    pub safety_flags: SourceSafetyFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRun {
    pub id: Uuid,
    #[serde(default)]
    pub seat: Option<SeatKind>,
    #[serde(default)]
    pub phase: Option<SessionPhase>,
    pub tool_name: String,
    pub input_summary: String,
    pub input_hash: String,
    pub status: String,
    pub duration_ms: u64,
    #[serde(default)]
    pub evidence_ids: Vec<Uuid>,
    #[serde(default)]
    pub error: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assessment {
    pub id: Uuid,
    pub assessor: SeatKind,
    pub evidence_id: Uuid,
    pub claim_id: Uuid,
    pub supports_claim: bool,
    pub reasoning: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimEvidenceLink {
    pub claim_id: Uuid,
    pub evidence_id: Uuid,
    pub link_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Critique {
    pub reviewer: SeatKind,
    pub target_seat: SeatKind,
    pub strongest_point: String,
    pub weakest_point: String,
    pub hidden_assumption: String,
    pub challenge: String,
    #[serde(default)]
    pub counterexample: String,
    pub suggested_improvement: String,
    #[serde(default)]
    pub evidence_question: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: Uuid,
    pub proposed_by: SeatKind,
    pub title: String,
    pub summary: String,
    pub source_idea_ids: Vec<Uuid>,
    #[serde(default)]
    pub adopted_points: Vec<String>,
    #[serde(default)]
    pub rejected_points: Vec<String>,
    #[serde(default)]
    pub rejection_reasons: Vec<String>,
    #[serde(default)]
    pub changes_from_initial: Vec<String>,
    pub user_value: String,
    pub implementation_path: String,
    pub risks: Vec<String>,
    pub success_metrics: Vec<String>,
    #[serde(default)]
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub voter: SeatKind,
    pub proposal_id: Uuid,
    pub value_score: u8,
    pub novelty_score: u8,
    pub feasibility_score: u8,
    pub risk_score: u8,
    pub roi_score: u8,
    pub final_choice: bool,
    pub reason: String,
    pub confidence: f32,
    #[serde(default)]
    pub key_evidence: String,
    #[serde(default)]
    pub blocking_issue: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub status: DecisionStatus,
    pub selected_proposal: Option<Proposal>,
    pub vote_count: usize,
    pub majority_reasons: Vec<String>,
    pub minority_opinion: Vec<String>,
    pub adoption_conditions: Vec<String>,
    pub unresolved_questions: Vec<String>,
    pub next_steps: Vec<String>,
    pub self_vote_count: usize,
    #[serde(default)]
    pub minority_choices: Vec<MinorityChoice>,
    #[serde(default)]
    pub reassessment_conditions: Vec<String>,
    #[serde(default)]
    pub has_risk_blocker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinorityChoice {
    pub seat: SeatKind,
    pub proposal_id: Uuid,
    pub reason: String,
    pub reassessment_condition: String,
    pub has_risk_warning: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    MajorityReached,
    ConditionallyAdopted,
    NoMajority,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoteStrategy {
    #[default]
    SimpleMajority,
    RiskVeto,
    Unanimous,
    ConditionalPass,
    WeightedScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotePolicy {
    pub allow_self_vote: bool,
    #[serde(default)]
    pub strategy: VoteStrategy,
    #[serde(default)]
    pub min_score_threshold: u8,
}

impl Default for VotePolicy {
    fn default() -> Self {
        Self {
            allow_self_vote: true,
            strategy: VoteStrategy::SimpleMajority,
            min_score_threshold: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoteOutcome {
    Majority(Uuid),
    NeedsConvergence,
    NoMajority,
}

pub fn phase_barrier_completed(results: &[(SeatKind, SeatStatus)]) -> Result<(), CoreError> {
    let completed: HashSet<_> = results
        .iter()
        .filter_map(|(seat, status)| (*status == SeatStatus::Completed).then_some(*seat))
        .collect();
    if SeatKind::ALL.iter().all(|seat| completed.contains(seat)) {
        Ok(())
    } else {
        Err(CoreError::PhaseBarrierNotSatisfied)
    }
}

pub fn tally_votes(
    proposals: &[Proposal],
    votes: &[Vote],
    policy: &VotePolicy,
    convergence_used: bool,
) -> Result<VoteOutcome, CoreError> {
    let proposal_authors: HashMap<Uuid, SeatKind> =
        proposals.iter().map(|p| (p.id, p.proposed_by)).collect();
    let mut final_votes: HashMap<Uuid, usize> = HashMap::new();
    let mut weighted_scores: HashMap<Uuid, f64> = HashMap::new();
    let mut valid_votes = 0usize;

    for vote in votes.iter().filter(|vote| vote.final_choice) {
        let Some(author) = proposal_authors.get(&vote.proposal_id) else {
            return Err(CoreError::UnknownProposal(vote.proposal_id));
        };
        if !policy.allow_self_vote && *author == vote.voter {
            continue;
        }
        if vote.reason.trim().is_empty() {
            return Err(CoreError::InvalidVote {
                seat: vote.voter,
                reason: "投票理由为空".into(),
            });
        }
        if vote.confidence < 0.1 || vote.confidence > 1.0 {
            return Err(CoreError::InvalidVote {
                seat: vote.voter,
                reason: format!("置信度不在有效范围: {}", vote.confidence),
            });
        }
        if !vote.blocking_issue.trim().is_empty() && policy.strategy == VoteStrategy::RiskVeto {
            return Err(CoreError::RiskVetoed {
                seat: vote.voter,
                issue: vote.blocking_issue.clone(),
            });
        }
        valid_votes += 1;
        *final_votes.entry(vote.proposal_id).or_default() += 1;
        *weighted_scores.entry(vote.proposal_id).or_default() += vote.value_score as f64
            + vote.novelty_score as f64
            + vote.feasibility_score as f64
            + vote.roi_score as f64
            - vote.risk_score as f64;
    }

    let majority_threshold = match policy.strategy {
        VoteStrategy::Unanimous => 3,
        _ => 2,
    };

    if valid_votes < 3 {
        return Ok(if convergence_used {
            VoteOutcome::NoMajority
        } else {
            VoteOutcome::NeedsConvergence
        });
    }

    if policy.strategy == VoteStrategy::WeightedScore
        && let Some((proposal_id, _)) = weighted_scores
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    {
        return Ok(VoteOutcome::Majority(proposal_id));
    }

    if let Some((proposal_id, _)) = final_votes
        .into_iter()
        .find(|(_, count)| *count >= majority_threshold)
    {
        Ok(VoteOutcome::Majority(proposal_id))
    } else if convergence_used {
        Ok(VoteOutcome::NoMajority)
    } else {
        Ok(VoteOutcome::NeedsConvergence)
    }
}

pub fn build_decision(
    proposals: &[Proposal],
    votes: &[Vote],
    policy: &VotePolicy,
    convergence_used: bool,
) -> Result<Decision, CoreError> {
    let outcome = tally_votes(proposals, votes, policy, convergence_used)?;
    let selected_id = match outcome {
        VoteOutcome::Majority(id) => Some(id),
        VoteOutcome::NeedsConvergence => None,
        VoteOutcome::NoMajority => None,
    };
    let selected_proposal =
        selected_id.and_then(|id| proposals.iter().find(|proposal| proposal.id == id).cloned());
    let self_vote_count = votes
        .iter()
        .filter(|vote| {
            vote.final_choice
                && proposals
                    .iter()
                    .find(|proposal| proposal.id == vote.proposal_id)
                    .is_some_and(|proposal| proposal.proposed_by == vote.voter)
        })
        .count();
    let minority_choices: Vec<MinorityChoice> = votes
        .iter()
        .filter(|vote| {
            vote.final_choice
                && selected_proposal
                    .as_ref()
                    .is_none_or(|proposal| vote.proposal_id != proposal.id)
        })
        .map(|vote| MinorityChoice {
            seat: vote.voter,
            proposal_id: vote.proposal_id,
            reason: vote.reason.clone(),
            reassessment_condition: if vote.blocking_issue.trim().is_empty() {
                "当多数方案实施后效果不达预期时应重新评估".into()
            } else {
                format!("当以下问题解决后应重新评估：{}", vote.blocking_issue)
            },
            has_risk_warning: !vote.blocking_issue.trim().is_empty(),
        })
        .collect();

    let has_risk_blocker = minority_choices.iter().any(|m| m.has_risk_warning);

    let status = if selected_proposal.is_some() {
        if has_risk_blocker {
            DecisionStatus::ConditionallyAdopted
        } else {
            DecisionStatus::MajorityReached
        }
    } else {
        DecisionStatus::NoMajority
    };
    let majority_reasons = selected_proposal
        .as_ref()
        .map(|proposal| {
            votes
                .iter()
                .filter(|vote| vote.final_choice && vote.proposal_id == proposal.id)
                .map(|vote| vote.reason.clone())
                .collect()
        })
        .unwrap_or_default();
    let minority_opinion = votes
        .iter()
        .filter(|vote| {
            selected_proposal
                .as_ref()
                .is_none_or(|proposal| vote.proposal_id != proposal.id)
        })
        .map(|vote| vote.reason.clone())
        .collect();

    let mut reassessment_conditions: Vec<String> = minority_choices
        .iter()
        .map(|m| m.reassessment_condition.clone())
        .collect();

    if let Some(selected) = &selected_proposal {
        for vote in votes
            .iter()
            .filter(|v| v.final_choice && v.proposal_id == selected.id)
        {
            if !vote.blocking_issue.trim().is_empty() {
                reassessment_conditions.push(format!("多数方也有关注：{}", vote.blocking_issue));
            }
        }
    }

    let adoption_conditions = selected_proposal
        .as_ref()
        .map(|proposal| {
            let mut conditions: Vec<String> = proposal
                .risks
                .iter()
                .map(|risk| format!("需控制风险：{risk}"))
                .collect();
            if policy.strategy == VoteStrategy::ConditionalPass {
                conditions.push("通过有条件多数决，需持续监控实施效果".into());
            }
            if conditions.is_empty() {
                conditions.push("多数策案已形成，建议优先执行".into());
            }
            conditions
        })
        .unwrap_or_default();

    let unresolved_questions = votes
        .iter()
        .filter(|vote| !vote.key_evidence.trim().is_empty())
        .map(|vote| format!("{} 认为关键证据：{}", vote.voter.label(), vote.key_evidence))
        .collect();

    Ok(Decision {
        status,
        selected_proposal,
        vote_count: votes.iter().filter(|vote| vote.final_choice).count(),
        majority_reasons,
        minority_opinion,
        adoption_conditions,
        unresolved_questions,
        next_steps: vec!["把多数方案拆成最小可执行清单".to_string()],
        self_vote_count: if policy.allow_self_vote {
            self_vote_count
        } else {
            0
        },
        minority_choices,
        reassessment_conditions,
        has_risk_blocker,
    })
}

pub fn generate_merged_proposal(proposals: &[Proposal], common_idea_ids: &[Uuid]) -> Proposal {
    let titles: Vec<&str> = proposals.iter().map(|p| p.title.as_str()).collect();
    let merged_title = format!("合案：{}", titles.join(" + "));
    let summaries: Vec<&str> = proposals.iter().map(|p| p.summary.as_str()).collect();
    let merged_summary = summaries.join("；");

    let mut all_adopted = Vec::new();
    let mut all_risks = Vec::new();
    let mut all_metrics = Vec::new();
    let mut all_implementation = Vec::new();
    let mut all_value = Vec::new();

    for proposal in proposals {
        all_adopted.extend(proposal.adopted_points.iter().cloned());
        all_risks.extend(proposal.risks.iter().cloned());
        all_metrics.extend(proposal.success_metrics.iter().cloned());
        all_implementation.push(proposal.implementation_path.clone());
        all_value.push(proposal.user_value.clone());
    }

    // Divergence analysis: identify disagreements
    let mut divergence_notes: Vec<String> = Vec::new();
    for (i, a) in proposals.iter().enumerate() {
        for b in proposals.iter().skip(i + 1) {
            let a_title = &a.title;
            let b_title = &b.title;
            if a.summary != b.summary {
                divergence_notes.push(format!(
                    "「{}」与「{}」策略方向不同：前者侧重「{}」，后者侧重「{}」",
                    a_title,
                    b_title,
                    a.summary.chars().take(30).collect::<String>(),
                    b.summary.chars().take(30).collect::<String>()
                ));
            }
            let a_risks: HashSet<&str> = a.risks.iter().map(|s| s.as_str()).collect();
            let b_risks: HashSet<&str> = b.risks.iter().map(|s| s.as_str()).collect();
            let unique_a: Vec<&&str> = a_risks.difference(&b_risks).collect();
            let unique_b: Vec<&&str> = b_risks.difference(&a_risks).collect();
            if !unique_a.is_empty() || !unique_b.is_empty() {
                divergence_notes.push(format!("「{}」与「{}」风险认知有差异", a_title, b_title));
            }
        }
    }

    let mut changes = vec!["合案复议：合并三席策案的共同部分".into()];
    if !divergence_notes.is_empty() {
        changes.push("核心分歧：".to_string());
        changes.extend(divergence_notes);
    }

    Proposal {
        id: Uuid::new_v4(),
        proposed_by: proposals
            .first()
            .map(|p| p.proposed_by)
            .unwrap_or(SeatKind::Mouyuan),
        title: merged_title,
        summary: merged_summary,
        source_idea_ids: common_idea_ids.to_vec(),
        adopted_points: all_adopted,
        rejected_points: vec![],
        rejection_reasons: vec![],
        changes_from_initial: changes,
        user_value: all_value.join("；"),
        implementation_path: all_implementation.join("；"),
        risks: all_risks,
        success_metrics: all_metrics,
        confidence: proposals.iter().map(|p| p.confidence).sum::<f32>() / proposals.len() as f32,
    }
}

// --- Search types ---

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("search request failed: {0}")]
    Request(String),
    #[error("search backend {0} returned error: {1}")]
    Backend(&'static str, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub snippet: String,
    pub url: String,
    pub source: String,
}

#[async_trait::async_trait]
pub trait SearchBackend: Send + Sync {
    fn name(&self) -> &'static str;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal(proposed_by: SeatKind) -> Proposal {
        Proposal {
            id: Uuid::new_v4(),
            proposed_by,
            title: format!("{} 策案", proposed_by.label()),
            summary: "summary".into(),
            source_idea_ids: vec![],
            adopted_points: vec![],
            rejected_points: vec![],
            rejection_reasons: vec![],
            changes_from_initial: vec![],
            user_value: "value".into(),
            implementation_path: "path".into(),
            risks: vec![],
            success_metrics: vec![],
            confidence: 0.8,
        }
    }

    #[test]
    fn legal_phase_transition_is_allowed() {
        let mut session = Session::new("t", "topic", "");
        assert!(
            session
                .transition_to(SessionPhase::IndependentDeliberation)
                .is_ok()
        );
        assert_eq!(session.phase, SessionPhase::IndependentDeliberation);
    }

    #[test]
    fn illegal_phase_transition_is_rejected() {
        let mut session = Session::new("t", "topic", "");
        assert_eq!(
            session.transition_to(SessionPhase::Voting).unwrap_err(),
            CoreError::IllegalPhaseTransition {
                from: SessionPhase::Draft,
                to: SessionPhase::Voting
            }
        );
    }

    #[test]
    fn independent_phase_waits_for_three_seats() {
        let results = [
            (SeatKind::Mouyuan, SeatStatus::Completed),
            (SeatKind::Jingshi, SeatStatus::Completed),
        ];
        assert_eq!(
            phase_barrier_completed(&results).unwrap_err(),
            CoreError::PhaseBarrierNotSatisfied
        );
    }

    #[test]
    fn single_seat_failure_does_not_pass_barrier() {
        let results = [
            (SeatKind::Mouyuan, SeatStatus::Completed),
            (SeatKind::Jingshi, SeatStatus::Failed),
            (SeatKind::Chizheng, SeatStatus::Completed),
        ];
        assert!(phase_barrier_completed(&results).is_err());
    }

    #[test]
    fn two_of_three_majority_selects_proposal() {
        let p1 = proposal(SeatKind::Mouyuan);
        let p2 = proposal(SeatKind::Jingshi);
        let proposals = vec![p1.clone(), p2];
        let votes = vec![
            vote(SeatKind::Mouyuan, p1.id),
            vote(SeatKind::Jingshi, p1.id),
            vote(SeatKind::Chizheng, p1.id),
        ];
        assert_eq!(
            tally_votes(&proposals, &votes, &VotePolicy::default(), false).unwrap(),
            VoteOutcome::Majority(p1.id)
        );
    }

    #[test]
    fn split_vote_enters_convergence_once() {
        let proposals = SeatKind::ALL.map(proposal);
        let votes = vec![
            vote(SeatKind::Mouyuan, proposals[0].id),
            vote(SeatKind::Jingshi, proposals[1].id),
            vote(SeatKind::Chizheng, proposals[2].id),
        ];
        assert_eq!(
            tally_votes(&proposals, &votes, &VotePolicy::default(), false).unwrap(),
            VoteOutcome::NeedsConvergence
        );
    }

    #[test]
    fn convergence_can_only_be_entered_once() {
        let mut session = Session::new("t", "topic", "");
        session
            .transition_to(SessionPhase::IndependentDeliberation)
            .unwrap();
        session.transition_to(SessionPhase::CrossCritique).unwrap();
        session.transition_to(SessionPhase::Revision).unwrap();
        session.transition_to(SessionPhase::Voting).unwrap();
        session.transition_to(SessionPhase::Convergence).unwrap();
        session.transition_to(SessionPhase::Voting).unwrap();
        assert_eq!(
            session
                .transition_to(SessionPhase::Convergence)
                .unwrap_err(),
            CoreError::ConvergenceAlreadyUsed
        );
    }

    #[test]
    fn second_split_vote_returns_no_majority() {
        let proposals = SeatKind::ALL.map(proposal);
        let votes = vec![
            vote(SeatKind::Mouyuan, proposals[0].id),
            vote(SeatKind::Jingshi, proposals[1].id),
            vote(SeatKind::Chizheng, proposals[2].id),
        ];
        assert_eq!(
            tally_votes(&proposals, &votes, &VotePolicy::default(), true).unwrap(),
            VoteOutcome::NoMajority
        );
    }

    fn vote(voter: SeatKind, proposal_id: Uuid) -> Vote {
        Vote {
            voter,
            proposal_id,
            value_score: 4,
            novelty_score: 4,
            feasibility_score: 4,
            risk_score: 3,
            roi_score: 4,
            final_choice: true,
            reason: "reason".into(),
            confidence: 0.8,
            key_evidence: String::new(),
            blocking_issue: String::new(),
        }
    }

    #[test]
    fn idea_status_default_is_proposed() {
        assert_eq!(IdeaStatus::default(), IdeaStatus::Proposed);
    }

    #[test]
    fn idea_card_default_status_is_proposed() {
        let card = IdeaCard {
            id: Uuid::new_v4(),
            proposed_by: SeatKind::Mouyuan,
            source_seats: vec![SeatKind::Mouyuan],
            title: "test".into(),
            summary: "test".into(),
            value: "test".into(),
            mechanism: "test".into(),
            unconventional: false,
            assumptions: vec![],
            risks: vec![],
            status: IdeaStatus::default(),
            challenged_by: vec![],
            referenced_by_proposals: vec![],
            merged_into: None,
        };
        assert_eq!(card.status, IdeaStatus::Proposed);
    }

    #[test]
    fn evidence_kind_default_is_fact() {
        assert_eq!(EvidenceKind::default(), EvidenceKind::Fact);
    }

    #[test]
    fn claim_evidence_links_roundtrip() {
        let claim_id = Uuid::new_v4();
        let evidence_id = Uuid::new_v4();
        let link = ClaimEvidenceLink {
            claim_id,
            evidence_id,
            link_type: "supports".into(),
        };
        assert_eq!(link.claim_id, claim_id);
        assert_eq!(link.evidence_id, evidence_id);
        assert_eq!(link.link_type, "supports");
    }

    #[test]
    fn unanimous_requires_three_votes() {
        let p1 = proposal(SeatKind::Mouyuan);
        let proposals = vec![p1.clone()];
        let votes = vec![
            vote(SeatKind::Mouyuan, p1.id),
            vote(SeatKind::Jingshi, p1.id),
        ];
        let policy = VotePolicy {
            allow_self_vote: true,
            strategy: VoteStrategy::Unanimous,
            min_score_threshold: 0,
        };
        assert_eq!(
            tally_votes(&proposals, &votes, &policy, false).unwrap(),
            VoteOutcome::NeedsConvergence
        );
    }

    #[test]
    fn unanimous_passes_with_three_same_votes() {
        let p1 = proposal(SeatKind::Mouyuan);
        let proposals = vec![p1.clone()];
        let votes = vec![
            vote(SeatKind::Mouyuan, p1.id),
            vote(SeatKind::Jingshi, p1.id),
            vote(SeatKind::Chizheng, p1.id),
        ];
        let policy = VotePolicy {
            allow_self_vote: true,
            strategy: VoteStrategy::Unanimous,
            min_score_threshold: 0,
        };
        assert_eq!(
            tally_votes(&proposals, &votes, &policy, false).unwrap(),
            VoteOutcome::Majority(p1.id)
        );
    }

    #[test]
    fn risk_veto_rejects_when_blocking_issue_present() {
        let p1 = proposal(SeatKind::Mouyuan);
        let proposals = vec![p1.clone()];
        let votes = vec![
            Vote {
                voter: SeatKind::Mouyuan,
                proposal_id: p1.id,
                final_choice: true,
                blocking_issue: "安全合规风险未解决".into(),
                ..vote(SeatKind::Mouyuan, p1.id)
            },
            vote(SeatKind::Jingshi, p1.id),
            vote(SeatKind::Chizheng, p1.id),
        ];
        let policy = VotePolicy {
            allow_self_vote: true,
            strategy: VoteStrategy::RiskVeto,
            min_score_threshold: 0,
        };
        assert_eq!(
            tally_votes(&proposals, &votes, &policy, false).unwrap_err(),
            CoreError::RiskVetoed {
                seat: SeatKind::Mouyuan,
                issue: "安全合规风险未解决".into()
            }
        );
    }

    #[test]
    fn risk_veto_passes_without_blocking_issue() {
        let p1 = proposal(SeatKind::Mouyuan);
        let proposals = vec![p1.clone()];
        let votes = vec![
            vote(SeatKind::Mouyuan, p1.id),
            vote(SeatKind::Jingshi, p1.id),
            vote(SeatKind::Chizheng, p1.id),
        ];
        let policy = VotePolicy {
            allow_self_vote: true,
            strategy: VoteStrategy::RiskVeto,
            min_score_threshold: 0,
        };
        assert_eq!(
            tally_votes(&proposals, &votes, &policy, false).unwrap(),
            VoteOutcome::Majority(p1.id)
        );
    }

    #[test]
    fn build_decision_includes_minority_choices() {
        let p1 = proposal(SeatKind::Mouyuan);
        let p2 = proposal(SeatKind::Jingshi);
        let proposals = vec![p1.clone(), p2.clone()];
        let votes = vec![
            vote(SeatKind::Mouyuan, p1.id),
            vote(SeatKind::Jingshi, p1.id),
            Vote {
                voter: SeatKind::Chizheng,
                proposal_id: p2.id,
                final_choice: true,
                blocking_issue: "数据隐私存疑".into(),
                reason: "隐私方案不完善".into(),
                ..vote(SeatKind::Chizheng, p2.id)
            },
        ];
        let decision = build_decision(&proposals, &votes, &VotePolicy::default(), false).unwrap();
        assert!(decision.selected_proposal.is_some());
        assert_eq!(decision.selected_proposal.as_ref().unwrap().id, p1.id);
        assert_eq!(decision.minority_choices.len(), 1);
        assert_eq!(decision.minority_choices[0].seat, SeatKind::Chizheng);
        assert!(decision.has_risk_blocker);
        assert!(!decision.reassessment_conditions.is_empty());
    }

    #[test]
    fn generate_merged_proposal_combines_all_proposals() {
        let p1 = proposal(SeatKind::Mouyuan);
        let p2 = proposal(SeatKind::Jingshi);
        let idea_ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let merged = generate_merged_proposal(&[p1, p2], &idea_ids);
        assert!(merged.title.contains("合案"));
        assert_eq!(merged.source_idea_ids.len(), 2);
        assert!(!merged.summary.is_empty());
    }

    #[test]
    fn conditional_pass_allows_two_third_majority() {
        let p1 = proposal(SeatKind::Mouyuan);
        let proposals = vec![p1.clone()];
        let votes = vec![
            vote(SeatKind::Mouyuan, p1.id),
            vote(SeatKind::Jingshi, p1.id),
            vote(SeatKind::Chizheng, p1.id),
        ];
        let policy = VotePolicy {
            allow_self_vote: true,
            strategy: VoteStrategy::ConditionalPass,
            min_score_threshold: 0,
        };
        assert_eq!(
            tally_votes(&proposals, &votes, &policy, false).unwrap(),
            VoteOutcome::Majority(p1.id)
        );
    }

    #[test]
    fn conditional_pass_adds_monitoring_condition() {
        let p1 = proposal(SeatKind::Mouyuan);
        let proposals = vec![p1.clone()];
        let votes = vec![
            vote(SeatKind::Mouyuan, p1.id),
            vote(SeatKind::Jingshi, p1.id),
            vote(SeatKind::Chizheng, p1.id),
        ];
        let policy = VotePolicy {
            allow_self_vote: true,
            strategy: VoteStrategy::ConditionalPass,
            min_score_threshold: 0,
        };
        let decision = build_decision(&proposals, &votes, &policy, false).unwrap();
        assert!(
            decision
                .adoption_conditions
                .iter()
                .any(|c| c.contains("持续监控"))
        );
    }

    #[test]
    fn tally_votes_rejects_unknown_proposal() {
        let proposals = vec![proposal(SeatKind::Mouyuan)];
        let votes = vec![Vote {
            voter: SeatKind::Mouyuan,
            proposal_id: Uuid::new_v4(),
            final_choice: true,
            ..vote(SeatKind::Mouyuan, Uuid::new_v4())
        }];
        assert_eq!(
            tally_votes(&proposals, &votes, &VotePolicy::default(), false).unwrap_err(),
            CoreError::UnknownProposal(votes[0].proposal_id)
        );
    }

    #[test]
    fn build_decision_with_key_evidence_adds_to_unresolved_questions() {
        let p1 = proposal(SeatKind::Mouyuan);
        let proposals = vec![p1.clone()];
        let votes = vec![
            Vote {
                voter: SeatKind::Mouyuan,
                proposal_id: p1.id,
                final_choice: true,
                key_evidence: "用户调研数据需补充".into(),
                ..vote(SeatKind::Mouyuan, p1.id)
            },
            vote(SeatKind::Jingshi, p1.id),
            vote(SeatKind::Chizheng, p1.id),
        ];
        let decision = build_decision(&proposals, &votes, &VotePolicy::default(), false).unwrap();
        assert!(
            decision
                .unresolved_questions
                .iter()
                .any(|q| q.contains("用户调研数据"))
        );
    }

    #[test]
    fn minority_choice_tracks_reassessment_condition_from_blocking_issue() {
        let p1 = proposal(SeatKind::Mouyuan);
        let p2 = proposal(SeatKind::Jingshi);
        let proposals = vec![p1.clone(), p2.clone()];
        let votes = vec![
            vote(SeatKind::Mouyuan, p1.id),
            vote(SeatKind::Jingshi, p1.id),
            Vote {
                voter: SeatKind::Chizheng,
                proposal_id: p2.id,
                final_choice: true,
                blocking_issue: "缺少安全审计".into(),
                reason: "安全审计未完成".into(),
                ..vote(SeatKind::Chizheng, p2.id)
            },
        ];
        let decision = build_decision(&proposals, &votes, &VotePolicy::default(), false).unwrap();
        let chizheng = decision
            .minority_choices
            .iter()
            .find(|m| m.seat == SeatKind::Chizheng)
            .unwrap();
        assert!(chizheng.reassessment_condition.contains("缺少安全审计"));
        assert!(chizheng.has_risk_warning);
        assert!(decision.has_risk_blocker);
    }
}
