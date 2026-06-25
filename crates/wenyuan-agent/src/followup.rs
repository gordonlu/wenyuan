use chrono::Utc;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use wenyuan_core::{
    DecisionObject, DecisionObjectKind, DecisionObjectPriority, DecisionObjectStatus,
    FollowUpKind, FollowUpImpact, FollowUpMode, FollowUpSuggestion, SeatKind, Session,
    SessionPhase,
};
use wenyuan_provider::{LlmProvider, LlmRequest, LlmResponse, ProviderError};

use crate::json::parse_model_json;
use crate::DiscussionArtifacts;

/// Generate decision objects from a completed session's artifacts.
/// Pure rule-based — no LLM calls.
pub fn generate_decision_objects(
    session: &Session,
    artifacts: &DiscussionArtifacts,
) -> Vec<DecisionObject> {
    let mut objects = Vec::new();
    let mut seen = HashSet::new();

    let decision = match artifacts.decision.as_ref() {
        Some(d) => d,
        None => return objects,
    };

    let session_id = session.id;

    // 1. Risks — from selected proposal, all proposals, decision fields
    if let Some(prop) = &decision.selected_proposal {
        for risk in &prop.risks {
            push_unique(
                &mut objects,
                &mut seen,
                DecisionObject {
                    id: Uuid::new_v4(),
                    session_id,
                    kind: DecisionObjectKind::Risk,
                    seat: Some(SeatKind::Chizheng),
                    title: truncate_title(risk),
                    summary: risk.clone(),
                    source_phase: Some(SessionPhase::Voting),
                    source_ref: Some(format!("proposal-{}-risks", prop.proposed_by.label())),
                    status: DecisionObjectStatus::Open,
                    priority: DecisionObjectPriority::High,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            );
        }
    }
    // Collect risks from all proposals
    for prop in &artifacts.proposals {
        for risk in &prop.risks {
            push_unique(
                &mut objects,
                &mut seen,
                DecisionObject {
                    id: Uuid::new_v4(),
                    session_id,
                    kind: DecisionObjectKind::Risk,
                    seat: source_seat_for_risk(prop.proposed_by),
                    title: truncate_title(risk),
                    summary: risk.clone(),
                    source_phase: Some(SessionPhase::Revision),
                    source_ref: Some(format!("proposal-{}-risks", prop.proposed_by.label())),
                    status: DecisionObjectStatus::Open,
                    priority: DecisionObjectPriority::Medium,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            );
        }
    }
    for condition in &decision.adoption_conditions {
        if contains_risk_keyword(condition) {
            push_unique(
                &mut objects,
                &mut seen,
                DecisionObject {
                    id: Uuid::new_v4(),
                    session_id,
                    kind: DecisionObjectKind::Risk,
                    seat: Some(SeatKind::Chizheng),
                    title: truncate_title(condition),
                    summary: condition.clone(),
                    source_phase: Some(SessionPhase::Completed),
                    source_ref: Some("adoption_conditions".into()),
                    status: DecisionObjectStatus::Open,
                    priority: DecisionObjectPriority::High,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            );
        }
    }
    for q in &decision.unresolved_questions {
        if contains_risk_keyword(q) {
            push_unique(
                &mut objects,
                &mut seen,
                DecisionObject {
                    id: Uuid::new_v4(),
                    session_id,
                    kind: DecisionObjectKind::Risk,
                    seat: Some(SeatKind::Chizheng),
                    title: truncate_title(q),
                    summary: q.clone(),
                    source_phase: Some(SessionPhase::Completed),
                    source_ref: Some("unresolved_questions".into()),
                    status: DecisionObjectStatus::Open,
                    priority: DecisionObjectPriority::High,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            );
        }
    }

    // 2. Assumptions — from IdeaCards
    for idea in &artifacts.ideas {
        for assumption in &idea.assumptions {
            let priority = if artifacts
                .decision
                .as_ref()
                .and_then(|d| d.selected_proposal.as_ref())
                .is_some_and(|p| {
                    p.source_idea_ids.contains(&idea.id)
                }) {
                DecisionObjectPriority::High
            } else {
                DecisionObjectPriority::Medium
            };
            push_unique(
                &mut objects,
                &mut seen,
                DecisionObject {
                    id: Uuid::new_v4(),
                    session_id,
                    kind: DecisionObjectKind::Assumption,
                    seat: Some(SeatKind::Chizheng),
                    title: truncate_title(assumption),
                    summary: assumption.clone(),
                    source_phase: Some(SessionPhase::IndependentDeliberation),
                    source_ref: Some(format!("idea-{}", idea.id)),
                    status: DecisionObjectStatus::Open,
                    priority,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            );
        }
    }

    // 3. Action items — from selected proposal
    if let Some(prop) = &decision.selected_proposal {
        if !prop.implementation_path.trim().is_empty() {
            push_unique(
                &mut objects,
                &mut seen,
                DecisionObject {
                    id: Uuid::new_v4(),
                    session_id,
                    kind: DecisionObjectKind::ActionItem,
                    seat: Some(SeatKind::Jingshi),
                    title: truncate_title(&prop.implementation_path),
                    summary: prop.implementation_path.clone(),
                    source_phase: Some(SessionPhase::Voting),
                    source_ref: Some("selected_proposal.implementation_path".into()),
                    status: DecisionObjectStatus::Open,
                    priority: DecisionObjectPriority::Medium,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            );
        }
        for metric in &prop.success_metrics {
            push_unique(
                &mut objects,
                &mut seen,
                DecisionObject {
                    id: Uuid::new_v4(),
                    session_id,
                    kind: DecisionObjectKind::ActionItem,
                    seat: Some(SeatKind::Jingshi),
                    title: truncate_title(metric),
                    summary: metric.clone(),
                    source_phase: Some(SessionPhase::Voting),
                    source_ref: Some("selected_proposal.success_metrics".into()),
                    status: DecisionObjectStatus::Open,
                    priority: DecisionObjectPriority::Medium,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            );
        }
    }
    for step in &decision.next_steps {
        push_unique(
            &mut objects,
            &mut seen,
            DecisionObject {
                id: Uuid::new_v4(),
                session_id,
                kind: DecisionObjectKind::ActionItem,
                seat: Some(SeatKind::Jingshi),
                title: truncate_title(step),
                summary: step.clone(),
                source_phase: Some(SessionPhase::Completed),
                source_ref: Some("decision.next_steps".into()),
                status: DecisionObjectStatus::Open,
                priority: DecisionObjectPriority::Low,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        );
    }

    // 4. Opportunities — from Mouyuan ideas, unconventional ideas
    for idea in &artifacts.ideas {
        if idea.proposed_by == SeatKind::Mouyuan || idea.unconventional {
            let priority = if idea.unconventional {
                DecisionObjectPriority::Low
            } else {
                DecisionObjectPriority::Medium
            };
            push_unique(
                &mut objects,
                &mut seen,
                DecisionObject {
                    id: Uuid::new_v4(),
                    session_id,
                    kind: DecisionObjectKind::Opportunity,
                    seat: Some(SeatKind::Mouyuan),
                    title: truncate_title(&idea.title),
                    summary: format!("{}：{}", idea.title, idea.summary),
                    source_phase: Some(SessionPhase::IndependentDeliberation),
                    source_ref: Some(format!("idea-{}", idea.id)),
                    status: DecisionObjectStatus::Open,
                    priority,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            );
        }
    }

    // 5. Open questions — from Decision.unresolved_questions (non-risk ones)
    for q in &decision.unresolved_questions {
        if !contains_risk_keyword(q) {
            push_unique(
                &mut objects,
                &mut seen,
                DecisionObject {
                    id: Uuid::new_v4(),
                    session_id,
                    kind: DecisionObjectKind::OpenQuestion,
                    seat: None,
                    title: truncate_title(q),
                    summary: q.clone(),
                    source_phase: Some(SessionPhase::Completed),
                    source_ref: Some("unresolved_questions".into()),
                    status: DecisionObjectStatus::Open,
                    priority: DecisionObjectPriority::Medium,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                },
            );
        }
    }

    // 6. Minority concerns — from Decision.minority_opinion, minority_choices
    for opinion in &decision.minority_opinion {
        let has_warning = decision
            .minority_choices
            .iter()
            .any(|mc| mc.has_risk_warning && mc.reason == *opinion);
        push_unique(
            &mut objects,
            &mut seen,
            DecisionObject {
                id: Uuid::new_v4(),
                session_id,
                kind: DecisionObjectKind::MinorityConcern,
                seat: None,
                title: truncate_title(opinion),
                summary: opinion.clone(),
                source_phase: Some(SessionPhase::Completed),
                source_ref: Some("minority_opinion".into()),
                status: DecisionObjectStatus::Open,
                priority: if has_warning {
                    DecisionObjectPriority::High
                } else {
                    DecisionObjectPriority::Medium
                },
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        );
    }
    for mc in &decision.minority_choices {
        let summary = format!("{}: {}", mc.seat.label(), mc.reason);
        push_unique(
            &mut objects,
            &mut seen,
            DecisionObject {
                id: Uuid::new_v4(),
                session_id,
                kind: DecisionObjectKind::MinorityConcern,
                seat: Some(mc.seat),
                title: truncate_title(&summary),
                summary,
                source_phase: Some(SessionPhase::Completed),
                source_ref: Some("minority_choices".into()),
                status: DecisionObjectStatus::Open,
                priority: if mc.has_risk_warning {
                    DecisionObjectPriority::High
                } else {
                    DecisionObjectPriority::Medium
                },
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        );
    }

    // Limit to 8 objects
    objects.truncate(8);
    objects
}

/// Run a single-seat follow-up expansion.
/// Calls the LLM for the given seat, parses the structured JSON output.
pub async fn run_single_seat_followup(
    provider: Arc<dyn LlmProvider>,
    session_id: Uuid,
    topic: &str,
    decision_summary: &str,
    object: &DecisionObject,
    seat: SeatKind,
    user_input: Option<&str>,
) -> Result<(serde_json::Value, FollowUpImpact), String> {
    let system_prompt = "你正在对一个已完成的文渊阁合议进行续议。请只围绕指定的决策对象展开，不要重跑完整合议，不要重复原报告。";

    let seat_label = seat.label();
    let object_kind_label = object.kind.label();
    let user_input_text = user_input.map(|u| format!("\n\n用户补充信息：\n{}", u)).unwrap_or_default();

    let user_prompt = format!(
        r#"原议题：
{}

原多数结论：
{}

当前续议对象：
类型：{object_kind_label}
来源席位：{seat_label}
内容：{}

请输出 JSON：
{{
  "summary": "本次续议的简短结论",
  "why_it_matters": "为什么这个对象重要",
  "analysis": "展开分析",
  "recommended_next_step": "下一步建议",
  "impact_on_original_decision": "no_change | clarifies | adds_condition | adds_action | raises_new_risk | suggests_redeliberation | changes_decision",
  "needs_user_input": false,
  "user_input_questions": []
}}{user_input_text}"#,
        topic,
        decision_summary,
        object.summary,
    );

    let response = call_followup_llm(provider, session_id, seat, &system_prompt, &user_prompt)
        .await
        .map_err(|e| format!("续议调用失败：{e}"))?;

    let parsed: serde_json::Value = parse_model_json(&response.content)
        .map_err(|e| format!("续议输出解析失败：{e}"))?;

    let impact_str = parsed
        .get("impact_on_original_decision")
        .and_then(|v| v.as_str())
        .unwrap_or("no_change");
    let impact = serde_json::from_str::<FollowUpImpact>(&format!("\"{impact_str}\""))
        .unwrap_or(FollowUpImpact::NoChange);

    Ok((parsed, impact))
}

async fn call_followup_llm(
    provider: Arc<dyn LlmProvider>,
    session_id: Uuid,
    seat: SeatKind,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<LlmResponse, ProviderError> {
    let request = LlmRequest {
        session_id,
        seat,
        phase: SessionPhase::Completed,
        messages: vec![
            wenyuan_core::ChatMessage {
                role: "system".into(),
                content: system_prompt.to_string(),
            },
            wenyuan_core::ChatMessage {
                role: "user".into(),
                content: user_prompt.to_string(),
            },
        ],
        repair_json: false,
        max_tokens: 2000,
        prompt_version: "followup-single-seat-v1".into(),
        reasoning_effort: None,
        override_model: None,
    };

    let mut last_err = Err(ProviderError::Request("no attempt".into()));
    let mut delay_ms = 500u64;
    for _ in 0..3 {
        let result = tokio::time::timeout(Duration::from_secs(60), provider.complete(request.clone())).await;
        match result {
            Ok(Ok(response)) => return Ok(response),
            Ok(Err(e)) => {
                last_err = Err(e);
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                delay_ms = (delay_ms * 2).min(8_000);
            }
            Err(_) => {
                last_err = Err(ProviderError::Timeout);
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                delay_ms = (delay_ms * 2).min(8_000);
            }
        }
    }
    last_err
}

/// Run a mini-deliberation follow-up.
/// Calls the LLM once (all three seats respond in the same prompt), parses positions.
pub async fn run_mini_deliberation(
    provider: Arc<dyn LlmProvider>,
    session_id: Uuid,
    topic: &str,
    decision_summary: &str,
    object: &DecisionObject,
    user_input: Option<&str>,
) -> Result<(serde_json::Value, FollowUpImpact), String> {
    let system_prompt = "你们正在围绕原合议中的一个局部对象进行小合议。每席只给一个短判断和一个关键理由。不要重跑完整独议、批议、复议、投票流程。";

    let user_input_text = user_input.map(|u| format!("\n\n用户补充信息：\n{}", u)).unwrap_or_default();

    let user_prompt = format!(
        r#"原议题：
{}

原多数结论：
{}

当前讨论对象：
类型：{}
内容：{}

请输出 JSON：
{{
  "question": "本次小合议要解决的问题",
  "positions": [
    {{"seat": "mouyuan", "view": "谋远席观点", "reason": "关键理由"}},
    {{"seat": "jingshi", "view": "经世席观点", "reason": "关键理由"}},
    {{"seat": "chizheng", "view": "持正席观点", "reason": "关键理由"}}
  ],
  "synthesis": "综合判断",
  "recommended_next_step": "建议下一步",
  "impact_on_original_decision": "no_change | clarifies | adds_condition | adds_action | raises_new_risk | suggests_redeliberation | changes_decision"
}}{user_input_text}"#,
        topic,
        decision_summary,
        object.kind.label(),
        object.summary,
    );

    let response = call_followup_llm(provider, session_id, SeatKind::Chizheng, system_prompt, &user_prompt)
        .await
        .map_err(|e| format!("小合议调用失败：{e}"))?;

    let parsed: serde_json::Value = parse_model_json(&response.content)
        .map_err(|e| format!("小合议输出解析失败：{e}"))?;

    let impact_str = parsed
        .get("impact_on_original_decision")
        .and_then(|v| v.as_str())
        .unwrap_or("no_change");
    let impact = serde_json::from_str::<FollowUpImpact>(&format!("\"{impact_str}\""))
        .unwrap_or(FollowUpImpact::NoChange);

    Ok((parsed, impact))
}

/// Run a new-fact re-deliberation.
/// Three seats each produce a short revision; backend builds updated decision.
pub async fn run_re_deliberation(
    provider: Arc<dyn LlmProvider>,
    session_id: Uuid,
    topic: &str,
    decision_summary: &str,
    new_fact: &str,
    affected_objects: &[DecisionObject],
) -> Result<(serde_json::Value, FollowUpImpact), String> {
    let system_prompt = "用户补充了一个可能影响原结论的新事实。请判断这个新事实是否改变原结论。三席分别给出修正意见，并形成更新结论。";

    let affected_text = if affected_objects.is_empty() {
        "无".into()
    } else {
        affected_objects
            .iter()
            .map(|o| format!("- {}（{}）", o.title, o.kind.label()))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let user_prompt = format!(
        r#"原议题：
{}

原多数结论：
{}

新事实：
{new_fact}

受影响的决策对象：
{affected_text}

请输出 JSON：
{{
  "new_fact": "{new_fact}",
  "fact_importance": "low | medium | high",
  "affected_objects": [],
  "seat_revisions": [
    {{"seat": "mouyuan", "revision": "修正意见"}},
    {{"seat": "jingshi", "revision": "修正意见"}},
    {{"seat": "chizheng", "revision": "修正意见"}}
  ],
  "decision_changed": true,
  "updated_decision": "更新后的结论",
  "why_changed": "为什么改变或不改变",
  "next_step": "下一步建议"
}}"#,
        topic,
        decision_summary,
    );

    let response = call_followup_llm(provider, session_id, SeatKind::Chizheng, system_prompt, &user_prompt)
        .await
        .map_err(|e| format!("新事实复议调用失败：{e}"))?;

    let parsed: serde_json::Value = parse_model_json(&response.content)
        .map_err(|e| format!("新事实复议输出解析失败：{e}"))?;

    let impact = if parsed.get("decision_changed").and_then(|v| v.as_bool()).unwrap_or(false) {
        FollowUpImpact::ChangesDecision
    } else {
        FollowUpImpact::NoChange
    };

    Ok((parsed, impact))
}

/// Generate follow-up suggestions from decision objects.
/// Max 3 suggestions, prioritized.
pub fn generate_followup_suggestions(objects: &[DecisionObject]) -> Vec<FollowUpSuggestion> {
    let mut suggestions = Vec::new();
    let mut seen = HashSet::new();

    // Priority order for suggestion kind
    let priority_order: &[(DecisionObjectKind, FollowUpKind, FollowUpMode, Option<SeatKind>)] = &[
        (
            DecisionObjectKind::Risk,
            FollowUpKind::MitigateRisk,
            FollowUpMode::SingleSeat,
            Some(SeatKind::Chizheng),
        ),
        (
            DecisionObjectKind::Risk,
            FollowUpKind::MitigateRisk,
            FollowUpMode::MiniDeliberation,
            None,
        ),
        (
            DecisionObjectKind::Assumption,
            FollowUpKind::VerifyAssumption,
            FollowUpMode::SingleSeat,
            Some(SeatKind::Chizheng),
        ),
        (
            DecisionObjectKind::ActionItem,
            FollowUpKind::BuildActionPlan,
            FollowUpMode::SingleSeat,
            Some(SeatKind::Jingshi),
        ),
        (
            DecisionObjectKind::Opportunity,
            FollowUpKind::ExpandOpportunity,
            FollowUpMode::SingleSeat,
            Some(SeatKind::Mouyuan),
        ),
        (
            DecisionObjectKind::Opportunity,
            FollowUpKind::ExpandOpportunity,
            FollowUpMode::MiniDeliberation,
            None,
        ),
        (
            DecisionObjectKind::MinorityConcern,
            FollowUpKind::DiscussMinorityConcern,
            FollowUpMode::MiniDeliberation,
            None,
        ),
        (
            DecisionObjectKind::OpenQuestion,
            FollowUpKind::ResolveOpenQuestion,
            FollowUpMode::SingleSeat,
            Some(SeatKind::Jingshi),
        ),
    ];

    // Sort objects by priority (Critical > High > Medium > Low)
    let mut sorted_objects: Vec<_> = objects.to_vec();
    sorted_objects.sort_by_key(|o| priority_rank(o.priority));

    for obj in &sorted_objects {
        if suggestions.len() >= 3 {
            break;
        }

        for &(kind, followup_kind, mode, _) in priority_order {
            if obj.kind != kind {
                continue;
            }
            let dedup_key = (kind, followup_kind, mode);
            if seen.contains(&dedup_key) {
                continue;
            }
            seen.insert(dedup_key);

            let session_id = obj.session_id;
            let (title, message, action_label) = suggestion_text(followup_kind, obj);

            suggestions.push(FollowUpSuggestion {
                id: Uuid::new_v4(),
                session_id,
                object_id: obj.id,
                kind: followup_kind,
                title,
                message,
                action_label,
                suggested_mode: mode,
                status: "open".into(),
                created_at: Utc::now(),
            });
            break;
        }
    }

    suggestions
}

fn push_unique(
    objects: &mut Vec<DecisionObject>,
    seen: &mut HashSet<String>,
    obj: DecisionObject,
) {
    let key = format!("{:?}|{}|{}", obj.kind, obj.title, obj.summary);
    if seen.insert(key) {
        objects.push(obj);
    }
}

fn truncate_title(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 40 {
        s.to_string()
    } else {
        let truncated: String = chars.into_iter().take(37).collect();
        format!("{}...", truncated)
    }
}

fn contains_risk_keyword(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.contains("风险")
        || lower.contains("危险")
        || lower.contains("隐患")
        || lower.contains("risk")
        || lower.contains("danger")
        || lower.contains("blocking")
        || lower.contains("veto")
        || lower.contains("不确定")
        || lower.contains("不清楚")
        || lower.contains("warning")
}

fn source_seat_for_risk(proposed_by: SeatKind) -> Option<SeatKind> {
    match proposed_by {
        SeatKind::Chizheng => Some(SeatKind::Chizheng),
        _ => Some(SeatKind::Chizheng), // Default to Chizheng for risk objects
    }
}

fn priority_rank(p: DecisionObjectPriority) -> u8 {
    match p {
        DecisionObjectPriority::Critical => 0,
        DecisionObjectPriority::High => 1,
        DecisionObjectPriority::Medium => 2,
        DecisionObjectPriority::Low => 3,
    }
}

fn suggestion_text(
    kind: FollowUpKind,
    obj: &DecisionObject,
) -> (String, String, String) {
    match kind {
        FollowUpKind::MitigateRisk => (
            format!("缓解风险：{}", obj.title),
            format!("{}席识别到一个关键风险：{}", obj.seat.map(|s| s.label()).unwrap_or("持正"), obj.summary),
            "查看风险缓解方案".into(),
        ),
        FollowUpKind::VerifyAssumption => (
            format!("验证假设：{}", obj.title),
            format!("当前结论依赖以下假设：{}", obj.summary),
            "验证假设".into(),
        ),
        FollowUpKind::BuildActionPlan => (
            format!("制定行动方案：{}", obj.title),
            format!("经世席建议的行动项：{}", obj.summary),
            "查看执行方案".into(),
        ),
        FollowUpKind::ExpandOpportunity => (
            format!("展开机会：{}", obj.title),
            format!("谋远席提出的机会方向：{}", obj.summary),
            "查看机会详情".into(),
        ),
        FollowUpKind::DiscussMinorityConcern => (
            format!("讨论少数意见：{}", obj.title),
            format!("少数留议中值得展开的分歧：{}", obj.summary),
            "查看少数意见".into(),
        ),
        FollowUpKind::ResolveOpenQuestion => (
            format!("澄清问题：{}", obj.title),
            format!("未决问题：{}", obj.summary),
            "查看分析".into(),
        ),
        FollowUpKind::ReDeliberateWithNewFact => (
            "新事实复议".into(),
            "补充新事实后重新评估原结论".into(),
            "提交新事实".into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wenyuan_core::{
        Decision, DecisionStatus, MinorityChoice, Proposal, Session,
    };

    fn make_session() -> Session {
        let mut s = Session::new("test", "议题", "");
        s.phase = SessionPhase::Completed;
        s
    }

    fn make_decision() -> Decision {
        Decision {
            status: DecisionStatus::MajorityReached,
            selected_proposal: Some(Proposal {
                id: Uuid::new_v4(),
                proposed_by: SeatKind::Chizheng,
                title: "持正方案".into(),
                summary: "持正席的建议方案".into(),
                source_idea_ids: vec![],
                adopted_points: vec![],
                rejected_points: vec![],
                rejection_reasons: vec![],
                changes_from_initial: vec![],
                user_value: "价值".into(),
                implementation_path: "分三步实施：调研、试点、推广".into(),
                risks: vec!["一次性调整多个变量，可能导致无法判断真实原因".into(), "预算可能不足".into()],
                success_metrics: vec!["用户满意度提升20%".into()],
                confidence: 0.8,
            }),
            vote_count: 3,
            majority_reasons: vec!["方案可行".into()],
            minority_opinion: vec!["谋远席认为风险较高".into()],
            adoption_conditions: vec!["需控制风险：数据隐私".into()],
            unresolved_questions: vec!["预算是否充足".into(), "法律风险待评估".into()],
            next_steps: vec!["把多数方案拆成最小可执行清单".into()],
            self_vote_count: 0,
            minority_choices: vec![MinorityChoice {
                seat: SeatKind::Mouyuan,
                proposal_id: Uuid::new_v4(),
                reason: "谋远席认为风险较高".into(),
                reassessment_condition: "当风险缓解后重新评估".into(),
                has_risk_warning: true,
            }],
            reassessment_conditions: vec![],
            has_risk_blocker: true,
        }
    }

    #[test]
    fn generates_objects_from_completed_session() {
        let session = make_session();
        let mut artifacts = DiscussionArtifacts::default();
        artifacts.decision = Some(make_decision());

        let objects = generate_decision_objects(&session, &artifacts);
        assert!(!objects.is_empty(), "should generate objects");
        assert!(objects.len() <= 8, "max 8 objects");
    }

    #[test]
    fn risk_objects_have_chizheng_seat() {
        let session = make_session();
        let mut artifacts = DiscussionArtifacts::default();
        artifacts.decision = Some(make_decision());

        let objects = generate_decision_objects(&session, &artifacts);
        for obj in &objects {
            if obj.kind == DecisionObjectKind::Risk {
                assert_eq!(obj.seat, Some(SeatKind::Chizheng));
            }
        }
    }

    #[test]
    fn action_items_route_to_jingshi() {
        let session = make_session();
        let mut artifacts = DiscussionArtifacts::default();
        artifacts.decision = Some(make_decision());

        let objects = generate_decision_objects(&session, &artifacts);
        for obj in &objects {
            if obj.kind == DecisionObjectKind::ActionItem {
                assert_eq!(obj.seat, Some(SeatKind::Jingshi));
            }
        }
    }

    #[test]
    fn generates_at_most_3_suggestions() {
        let session = make_session();
        let mut artifacts = DiscussionArtifacts::default();
        artifacts.decision = Some(make_decision());

        let objects = generate_decision_objects(&session, &artifacts);
        assert!(!objects.is_empty());

        let suggestions = generate_followup_suggestions(&objects);
        assert!(suggestions.len() <= 3, "max 3 suggestions, got {}", suggestions.len());
        assert!(!suggestions.is_empty(), "should have at least 1 suggestion");
    }

    #[test]
    fn high_risk_objects_prioritized() {
        let session = make_session();
        let mut artifacts = DiscussionArtifacts::default();
        artifacts.decision = Some(make_decision());

        let objects = generate_decision_objects(&session, &artifacts);
        let suggestions = generate_followup_suggestions(&objects);

        // First suggestion should be risk-related
        if let Some(first) = suggestions.first() {
            assert_eq!(first.kind, FollowUpKind::MitigateRisk);
        }
    }

    #[test]
    fn limited_to_8_objects() {
        let session = make_session();
        let mut artifacts = DiscussionArtifacts::default();

        // Create a decision with many items
        let mut decision = make_decision();
        // Add more risks to overload
        if let Some(ref mut prop) = decision.selected_proposal {
            for i in 0..20 {
                prop.risks.push(format!("风险条目 {}", i));
            }
        }
        artifacts.decision = Some(decision);

        let objects = generate_decision_objects(&session, &artifacts);
        assert!(objects.len() <= 8, "should cap at 8, got {}", objects.len());
    }

    #[test]
    fn empty_session_returns_empty() {
        let session = Session::new("test", "议题", "");
        let artifacts = DiscussionArtifacts::default();
        let objects = generate_decision_objects(&session, &artifacts);
        assert!(objects.is_empty());
    }
}
