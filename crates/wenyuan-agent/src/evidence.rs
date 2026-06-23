use uuid::Uuid;
use wenyuan_core::{
    Claim, ClaimEvidenceLink, Evidence, EvidenceSourceKind, EvidenceTrustLevel, IdeaCard,
    IdeaStatus, Proposal, SeatKind, SourceSafetyFlags,
};

pub(crate) fn compute_idea_statuses(
    ideas: &mut [IdeaCard],
    critiques: &[wenyuan_core::Critique],
    proposals: &[Proposal],
    final_decision: Option<&wenyuan_core::Decision>,
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

pub(crate) fn extract_evidence_pool(
    ideas: &[IdeaCard],
    critiques: &[wenyuan_core::Critique],
    _proposals: &[Proposal],
) -> (
    Vec<Claim>,
    Vec<Evidence>,
    Vec<wenyuan_core::Assessment>,
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
