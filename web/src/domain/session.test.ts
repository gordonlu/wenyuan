import { describe, expect, it } from 'vitest'
import { exportSessionMarkdown, hasMajority, qualityMetricRows, revisionDiffs, seatRunStats, seatStatus, type SessionDetails } from './session'

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

    const markdown = exportSessionMarkdown(details)
    expect(markdown).toContain('# 增长策略')
    expect(markdown).toContain('## 原始议题')
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
