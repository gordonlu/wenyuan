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

    pub fn label(self) -> &'static str {
        match self {
            SeatKind::Mouyuan => "谋远席",
            SeatKind::Jingshi => "经世席",
            SeatKind::Chizheng => "持正席",
        }
    }
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
pub struct Session {
    pub id: Uuid,
    pub title: String,
    pub topic: String,
    pub context: String,
    pub phase: SessionPhase,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub result: Option<Decision>,
    pub failure_reason: Option<String>,
    pub convergence_used: bool,
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
            phase: SessionPhase::Draft,
            created_at: now,
            updated_at: now,
            result: None,
            failure_reason: None,
            convergence_used: false,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaCard {
    pub id: Uuid,
    pub proposed_by: SeatKind,
    pub title: String,
    pub summary: String,
    pub value: String,
    pub mechanism: String,
    pub assumptions: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Critique {
    pub reviewer: SeatKind,
    pub target_seat: SeatKind,
    pub strongest_point: String,
    pub weakest_point: String,
    pub hidden_assumption: String,
    pub challenge: String,
    pub suggested_improvement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: Uuid,
    pub proposed_by: SeatKind,
    pub title: String,
    pub summary: String,
    pub source_idea_ids: Vec<Uuid>,
    pub user_value: String,
    pub implementation_path: String,
    pub risks: Vec<String>,
    pub success_metrics: Vec<String>,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    MajorityReached,
    NoMajority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotePolicy {
    pub allow_self_vote: bool,
}

impl Default for VotePolicy {
    fn default() -> Self {
        Self {
            allow_self_vote: true,
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
    let mut valid_votes = 0usize;

    for vote in votes.iter().filter(|vote| vote.final_choice) {
        let Some(author) = proposal_authors.get(&vote.proposal_id) else {
            return Err(CoreError::UnknownProposal(vote.proposal_id));
        };
        if !policy.allow_self_vote && *author == vote.voter {
            continue;
        }
        valid_votes += 1;
        *final_votes.entry(vote.proposal_id).or_default() += 1;
    }

    if valid_votes < 3 {
        return Ok(if convergence_used {
            VoteOutcome::NoMajority
        } else {
            VoteOutcome::NeedsConvergence
        });
    }

    if let Some((proposal_id, _)) = final_votes.into_iter().find(|(_, count)| *count >= 2) {
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
    let status = if selected_proposal.is_some() {
        DecisionStatus::MajorityReached
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

    Ok(Decision {
        status,
        selected_proposal,
        vote_count: votes.iter().filter(|vote| vote.final_choice).count(),
        majority_reasons,
        minority_opinion,
        adoption_conditions: vec!["先以 Mock 模式验证完整讨论链路".to_string()],
        unresolved_questions: vec!["真实模型输出稳定性需要继续观察".to_string()],
        next_steps: vec!["把多数方案拆成最小可执行清单".to_string()],
        self_vote_count: if policy.allow_self_vote {
            self_vote_count
        } else {
            0
        },
    })
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
            user_value: "value".into(),
            implementation_path: "path".into(),
            risks: vec![],
            success_metrics: vec![],
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
        }
    }
}
