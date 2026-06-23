import { describe, expect, it } from 'vitest'
import { evidenceSummary, decisionDigest, exportSessionMarkdown, hasMajority, qualityMetricRows, revisionDiffs, seatRunStats, seatStatus, type SessionDetails } from './session'

describe('session domain helpers', () => {
  it('detects final majority display state', () => {
    const details = {
      session: {
        id: 's1',
        title: 't',
        topic: 'topic',
        context: '',
        mode: 'three_seat',
        phase: 'completed',
        result: {
          status: 'majority_reached',
          vote_count: 2,
          majority_reasons: [],
          minority_opinion: [],
          adoption_conditions: [],
          unresolved_questions: [],
          next_steps: [],
          self_vote_count: 0,
        },
      },
      artifacts: { ideas: [], critiques: [], proposals: [], votes: [], seat_runs: [], events: [] },
      execution: { running: false, recovery_state: 'completed' },
      events: [],
    } as SessionDetails
    expect(hasMajority(details)).toBe(true)
  })

  it('maps failed seat status from events', () => {
    expect(
      seatStatus('revision', 'jingshi', [
        { id: 1, event_type: 'seat_failed', payload: { seat: 'jingshi' }, created_at: new Date().toISOString() },
      ]),
    ).toBe('失败')
  })

  it('maps running and completed seat status from progress events', () => {
    expect(
      seatStatus('cross_critique', 'mouyuan', [
        { id: 1, event_type: 'seat_started', payload: { seat: 'mouyuan', phase: 'cross_critique' }, created_at: new Date().toISOString() },
      ]),
    ).toBe('进行中')
    expect(
      seatStatus('cross_critique', 'mouyuan', [
        { id: 1, event_type: 'seat_started', payload: { seat: 'mouyuan', phase: 'cross_critique' }, created_at: new Date().toISOString() },
        { id: 2, event_type: 'seat_completed', payload: { seat: 'mouyuan', phase: 'cross_critique' }, created_at: new Date().toISOString() },
      ]),
    ).toBe('已完成')
  })

  it('aggregates seat run token and failure stats', () => {
    const stats = seatRunStats([
      {
        id: 'r1',
        session_id: 's1',
        seat: 'mouyuan',
        phase: 'voting',
        status: 'failed',
        prompt_version: 'mouyuan-v1',
        repair_attempted: false,
        duration_ms: 12,
        total_tokens: 20,
      },
      {
        id: 'r2',
        session_id: 's1',
        seat: 'mouyuan',
        phase: 'voting',
        status: 'completed',
        prompt_version: 'mouyuan-v1',
        repair_attempted: true,
        duration_ms: 30,
        total_tokens: 40,
      },
    ])
    expect(stats[0]).toMatchObject({ calls: 2, failed: 1, repaired: 1, tokens: 60, durationMs: 42 })
  })

  it('exports discussion details as markdown', () => {
    const details = {
      session: {
        id: 's1',
        title: '增长策略',
        topic: '如何提升留存',
        context: '面向新用户',
        mode: 'three_seat',
        phase: 'completed',
        result: {
          status: 'majority_reached',
          vote_count: 2,
          majority_reasons: ['价值明确'],
          minority_opinion: ['需要控制成本'],
          adoption_conditions: ['两周复盘'],
          unresolved_questions: ['预算上限'],
          next_steps: ['启动实验'],
          self_vote_count: 1,
          selected_proposal: {
            id: 'p1',
            proposed_by: 'mouyuan',
            title: '分层引导',
            summary: '按用户意图拆分路径',
            user_value: '更快完成首次价值体验',
            implementation_path: '先做轻量实验',
            risks: ['样本不足'],
            success_metrics: ['次日留存提升'],
          },
        },
      },
      artifacts: {
        ideas: [
          {
            id: 'i1',
            proposed_by: 'mouyuan',
            title: '激活路径',
            summary: '缩短首次成功路径',
            value: '降低流失',
            mechanism: '分层触达',
            assumptions: ['用户目标可识别'],
            risks: ['打扰用户'],
          },
        ],
        critiques: [
          {
            reviewer: 'jingshi',
            target_seat: 'mouyuan',
            strongest_point: '价值清晰',
            weakest_point: '成本未知',
            hidden_assumption: '有足够数据',
            challenge: '明确实验边界',
            suggested_improvement: '限定人群',
          },
        ],
        proposals: [
          {
            id: 'p1',
            proposed_by: 'mouyuan',
            title: '分层引导',
            summary: '按用户意图拆分路径',
            user_value: '更快完成首次价值体验',
            implementation_path: '先做轻量实验',
            risks: ['样本不足'],
            success_metrics: ['次日留存提升'],
          },
        ],
        votes: [{ voter: 'jingshi', proposal_id: 'p1', final_choice: true, reason: '可落地' }],
        seat_runs: [],
        events: [],
      },
      execution: { running: false, recovery_state: 'completed' },
      events: [],
    } as SessionDetails

    const markdown = exportSessionMarkdown(details, 'standard')
    expect(markdown).toContain('# 增长策略')
    expect(markdown).toContain('## 2. 议题与背景')
    expect(markdown).toContain('### 谋远席：激活路径')
    expect(markdown).toContain('### 经世席 → 谋远席')
    expect(markdown).toContain('### 谋远席：分层引导')
    expect(markdown).toContain('- 少数留议：')
    expect(markdown).toContain('- 经世席：支持 分层引导。可落地')
  })

  it('compares independent ideas with revised proposals', () => {
    const details = {
      session: { id: 's1', title: 't', topic: 'topic', context: '', mode: 'three_seat', phase: 'completed' },
      artifacts: {
        ideas: [
          { id: 'i1', proposed_by: 'mouyuan', title: '宽口径增长', summary: '扩大触达', value: '更多线索' },
          { id: 'i2', proposed_by: 'mouyuan', title: '低成本实验', summary: '小样本测试', value: '控制风险' },
        ],
        critiques: [],
        proposals: [
          {
            id: 'p1',
            proposed_by: 'mouyuan',
            title: '低成本分层实验',
            summary: '先用小样本测试重点人群',
            source_idea_ids: ['i2'],
            user_value: '控制风险并验证留存',
            implementation_path: '两周内完成 A/B 实验',
            risks: [],
            success_metrics: ['留存提升'],
          },
        ],
        votes: [],
        seat_runs: [],
        events: [],
      },
      execution: { running: false, recovery_state: 'completed' },
      events: [],
    } as SessionDetails

    expect(revisionDiffs(details)[0]).toMatchObject({
      seat: 'mouyuan',
      ideaTitles: ['低成本实验'],
      proposalTitle: '低成本分层实验',
      titleChanged: true,
      summaryChanged: true,
      adoptedIdeaCount: 1,
      addedImplementationPath: '两周内完成 A/B 实验',
      addedSuccessMetrics: ['留存提升'],
    })
  })

  it('evidenceSummary returns zero counts for empty evidence', () => {
    const details = {
      session: { id: 's1', title: 't', topic: '', context: '', mode: 'three_seat', phase: 'draft' },
      artifacts: { ideas: [], critiques: [], proposals: [], votes: [], seat_runs: [], events: [], evidence: [], claims: [] },
      execution: { running: false, recovery_state: 'idle' },
      events: [],
    } as unknown as SessionDetails
    const summary = evidenceSummary(details)
    expect(summary.total).toBe(0)
    expect(summary.untrusted_count).toBe(0)
    expect(summary.injection_risk_count).toBe(0)
    expect(summary.unverified_claims).toBe(0)
    expect(summary.has_safety_warnings).toBe(false)
  })

  it('evidenceSummary counts internal-only evidence', () => {
    const details = {
      session: { id: 's1', title: 't', topic: '', context: '', mode: 'three_seat', phase: 'completed' },
      artifacts: {
        ideas: [], critiques: [], proposals: [], votes: [], seat_runs: [], events: [],
        evidence: [
          { id: 'e1', content: 'idea inference', source: '谋远席 独议', source_kind: 'internal', trust_level: 'internal' },
          { id: 'e2', content: 'idea risk', source: '经世席 独议', source_kind: 'internal', trust_level: 'internal' },
        ],
        claims: [],
      },
      execution: { running: false, recovery_state: 'completed' },
      events: [],
    } as unknown as SessionDetails
    const summary = evidenceSummary(details)
    expect(summary.total).toBe(2)
    expect(summary.by_source.internal).toBe(2)
    expect(summary.untrusted_count).toBe(0)
  })

  it('evidenceSummary detects external sources and safety flags', () => {
    const details = {
      session: { id: 's1', title: 't', topic: '', context: '', mode: 'three_seat', phase: 'completed' },
      artifacts: {
        ideas: [], critiques: [], proposals: [], votes: [], seat_runs: [], events: [],
        evidence: [
          { id: 'e1', content: 'web result', source: 'https://example.com', source_kind: 'web_search', trust_level: 'untrusted_external', safety_flags: {} },
          { id: 'e2', content: 'file result', source: 'doc.pdf', source_kind: 'file', trust_level: 'untrusted_external', safety_flags: { prompt_injection_risk: true } },
          { id: 'e3', content: 'internal evidence', source: '持正席 独议', source_kind: 'internal', trust_level: 'internal' },
        ],
        claims: [
          { id: 'c1', content: 'claim1', is_supported: true },
          { id: 'c2', content: 'claim2', is_supported: false },
        ],
      },
      execution: { running: false, recovery_state: 'completed' },
      events: [],
    } as unknown as SessionDetails
    const summary = evidenceSummary(details)
    expect(summary.total).toBe(3)
    expect(summary.by_source.web_search).toBe(1)
    expect(summary.by_source.file).toBe(1)
    expect(summary.by_source.internal).toBe(1)
    expect(summary.untrusted_count).toBe(2)
    expect(summary.injection_risk_count).toBe(1)
    expect(summary.unverified_claims).toBe(1)
    expect(summary.has_safety_warnings).toBe(true)
  })

  it('decisionDigest returns no-decision state', () => {
    const details = {
      session: { id: 's1', title: 't', topic: '', context: '', mode: 'three_seat', phase: 'draft' },
      artifacts: { ideas: [], critiques: [], proposals: [], votes: [], seat_runs: [], events: [], evidence: [], claims: [] },
      execution: { running: false, recovery_state: 'idle' },
      events: [],
    } as unknown as SessionDetails
    const digest = decisionDigest(details)
    expect(digest.has_decision).toBe(false)
    expect(digest.status_label).toBe('尚无结论')
  })

  it('decisionDigest extracts majority decision fields', () => {
    const details = {
      session: {
        id: 's1', title: 't', topic: 'topic', context: '', mode: 'three_seat', phase: 'completed',
        result: {
          status: 'majority_reached',
          vote_count: 2,
          majority_reasons: ['方案可行', '成本可控'],
          minority_opinion: ['需要更多数据'],
          adoption_conditions: ['两周内验证'],
          next_steps: ['启动A/B实验'],
          self_vote_count: 1,
          selected_proposal: { id: 'p1', proposed_by: 'mouyuan', title: '分层实验', summary: 'test' },
          has_risk_blocker: false,
          minority_choices: [],
        },
      },
      artifacts: { ideas: [], critiques: [], proposals: [], votes: [], seat_runs: [], events: [], evidence: [], claims: [] },
      execution: { running: false, recovery_state: 'completed' },
      events: [],
    } as unknown as SessionDetails
    const digest = decisionDigest(details)
    expect(digest.has_decision).toBe(true)
    expect(digest.status_label).toBe('形成多数')
    expect(digest.status_class).toBe('ok')
    expect(digest.selected_proposal_title).toBe('分层实验')
    expect(digest.selected_proposal_seat).toBe('mouyuan')
    expect(digest.majority_reason_summary).toContain('方案可行')
  })

  it('decisionDigest detects risk blocker and conditionally adopted', () => {
    const details = {
      session: {
        id: 's1', title: 't', topic: 'topic', context: '', mode: 'three_seat', phase: 'completed',
        result: {
          status: 'conditionally_adopted',
          vote_count: 2,
          majority_reasons: ['价值明确'],
          minority_opinion: [],
          adoption_conditions: ['需解决安全合规'],
          next_steps: ['启动安全性审计'],
          self_vote_count: 0,
          selected_proposal: { id: 'p1', proposed_by: 'chizheng', title: '安全优先方案', summary: 'test' },
          has_risk_blocker: true,
          minority_choices: [{ seat: 'chizheng', proposal_id: 'p1', reason: '隐私风险未解决', has_risk_warning: true }],
        },
      },
      artifacts: { ideas: [], critiques: [], proposals: [], votes: [], seat_runs: [], events: [], evidence: [], claims: [] },
      execution: { running: false, recovery_state: 'completed' },
      events: [],
    } as unknown as SessionDetails
    const digest = decisionDigest(details)
    expect(digest.has_decision).toBe(true)
    expect(digest.status_label).toBe('有条件通过')
    expect(digest.status_class).toBe('warn')
    expect(digest.has_risk_blocker).toBe(true)
    expect(digest.minority_count).toBe(1)
  })

  it('formats quality metric rows', () => {
    expect(
      qualityMetricRows({
        idea_duplicate_rate: 0.25,
        seat_similarity: 0.5,
        high_similarity_detected: false,
        critique_effectiveness_rate: 1,
        revision_change_rate: 0.75,
        self_vote_rate: 0.33,
        vote_concentration: 0.67,
        minority_retention_rate: 0.33,
        average_tokens: 123.4,
        average_duration_ms: 56.7,
      }),
    ).toContainEqual({ label: '批议有效率', value: '100%' })
  })
})
