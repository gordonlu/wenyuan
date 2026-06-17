import { describe, expect, it } from 'vitest'
import { hasMajority, seatRunStats, seatStatus, type SessionDetails } from './session'

describe('session domain helpers', () => {
  it('detects final majority display state', () => {
    const details = {
      session: {
        id: 's1',
        title: 't',
        topic: 'topic',
        context: '',
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
})
