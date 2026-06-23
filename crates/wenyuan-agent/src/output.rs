use serde::Deserialize;
use uuid::Uuid;
use wenyuan_core::{Evidence, SeatKind, ToolRun};

pub(crate) struct PhaseRun<T> {
    pub(crate) outputs: Vec<(SeatKind, T)>,
    pub(crate) traces: Vec<super::SeatRunTrace>,
    pub(crate) evidence: Vec<Evidence>,
    pub(crate) tool_runs: Vec<ToolRun>,
}

pub(crate) struct SeatCallResult<T> {
    pub(crate) seat: SeatKind,
    pub(crate) parsed: Option<T>,
    pub(crate) traces: Vec<super::SeatRunTrace>,
    pub(crate) evidence: Vec<Evidence>,
    pub(crate) tool_runs: Vec<ToolRun>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct IndependentOutput {
    pub(crate) ideas: Vec<RawIdea>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawIdea {
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) title: String,
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) summary: String,
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) value: String,
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) mechanism: String,
    #[serde(default)]
    pub(crate) unconventional: bool,
    #[serde(default)]
    pub(crate) assumptions: Vec<String>,
    #[serde(default)]
    pub(crate) risks: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CritiqueOutput {
    pub(crate) reviews: Vec<RawCritique>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawCritique {
    #[serde(alias = "seat")]
    pub(crate) target_seat: SeatKind,
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) strongest_point: String,
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) weakest_point: String,
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) hidden_assumption: String,
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) challenge: String,
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) counterexample: String,
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) suggested_improvement: String,
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) evidence_question: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProposalOutput {
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) title: String,
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) summary: String,
    #[serde(default, deserialize_with = "crate::json::deserialize_uuid_vec_lossy")]
    pub(crate) source_idea_ids: Vec<Uuid>,
    #[serde(default)]
    pub(crate) adopted_points: Vec<String>,
    #[serde(default)]
    pub(crate) rejected_points: Vec<String>,
    #[serde(default)]
    pub(crate) rejection_reasons: Vec<String>,
    #[serde(default)]
    pub(crate) changes_from_initial: Vec<String>,
    #[serde(
        default,
        alias = "value",
        alias = "user_benefit",
        deserialize_with = "crate::json::deserialize_string_loose"
    )]
    pub(crate) user_value: String,
    #[serde(
        default,
        alias = "implementation_plan",
        alias = "action_plan",
        deserialize_with = "crate::json::deserialize_string_loose"
    )]
    pub(crate) implementation_path: String,
    #[serde(default)]
    pub(crate) risks: Vec<String>,
    #[serde(default, alias = "metrics", alias = "success_criteria")]
    pub(crate) success_metrics: Vec<String>,
    #[serde(default)]
    pub(crate) confidence: f32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct VoteOutput {
    pub(crate) votes: Vec<RawVote>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawVote {
    #[serde(
        default,
        alias = "proposal_id",
        alias = "proposal",
        alias = "choice",
        deserialize_with = "crate::json::deserialize_string_loose"
    )]
    pub(crate) proposal_ref: String,
    #[serde(default)]
    pub(crate) value_score: f32,
    #[serde(default)]
    pub(crate) novelty_score: f32,
    #[serde(default)]
    pub(crate) feasibility_score: f32,
    #[serde(default)]
    pub(crate) risk_score: f32,
    #[serde(default)]
    pub(crate) roi_score: f32,
    #[serde(default, deserialize_with = "crate::json::deserialize_boolish")]
    pub(crate) final_choice: bool,
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) confidence: f32,
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) key_evidence: String,
    #[serde(default, deserialize_with = "crate::json::deserialize_string_loose")]
    pub(crate) blocking_issue: String,
}
