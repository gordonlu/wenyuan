export type SeatKind = 'mouyuan' | 'jingshi' | 'chizheng'
export type SessionPhase =
  | 'draft'
  | 'independent_deliberation'
  | 'cross_critique'
  | 'revision'
  | 'voting'
  | 'convergence'
  | 'completed'
  | 'failed'
  | 'cancelled'

export interface Proposal {
  id: string
  proposed_by: SeatKind
  title: string
  summary: string
  user_value: string
  implementation_path: string
  risks: string[]
  success_metrics: string[]
}

export interface Decision {
  status: 'majority_reached' | 'no_majority'
  selected_proposal?: Proposal | null
  vote_count: number
  majority_reasons: string[]
  minority_opinion: string[]
  adoption_conditions: string[]
  unresolved_questions: string[]
  next_steps: string[]
  self_vote_count: number
}

export interface SessionRecord {
  id: string
  title: string
  topic: string
  context: string
  phase: SessionPhase
  result?: Decision | null
  failure_reason?: string | null
}

export interface SessionSummary {
  id: string
  title: string
  phase: SessionPhase
  created_at: string
  updated_at: string
  has_majority: boolean
}

export interface SessionEvent {
  id: number
  event_type: string
  payload: unknown
  created_at: string
}

export interface DiscussionArtifacts {
  ideas: Array<{ id: string; proposed_by: SeatKind; title: string; summary: string; value: string }>
  critiques: Array<{ reviewer: SeatKind; target_seat: SeatKind; challenge: string; suggested_improvement: string }>
  proposals: Proposal[]
  votes: Array<{ voter: SeatKind; proposal_id: string; final_choice: boolean; reason: string }>
  seat_runs: SeatRunTrace[]
  decision?: Decision | null
  events: string[]
}

export interface SeatRunTrace {
  id: string
  session_id: string
  seat: SeatKind
  phase: SessionPhase
  status: 'completed' | 'failed'
  prompt_version: string
  repair_attempted: boolean
  raw_output?: string | null
  error?: string | null
  duration_ms: number | string
  prompt_tokens?: number | null
  completion_tokens?: number | null
  total_tokens?: number | null
  upstream_status?: number | null
}

export interface SessionDetails {
  session: SessionRecord
  artifacts: DiscussionArtifacts
  execution: ExecutionInfo
  events: SessionEvent[]
}

export interface ExecutionInfo {
  running: boolean
  lease_expires_at?: string | null
  recovery_state: 'idle' | 'running' | 'completed' | 'failed' | 'cancelled' | 'retry_required' | string
}

export interface ConfigStatus {
  provider_configured: boolean
  provider_kind: string
  model: string
  database_url: string
  version: string
}

export const seatLabels: Record<SeatKind, string> = {
  mouyuan: '谋远席',
  jingshi: '经世席',
  chizheng: '持正席',
}

export const phaseLabels: Record<SessionPhase, string> = {
  draft: '待陈策',
  independent_deliberation: '独议中',
  cross_critique: '批议中',
  revision: '复议中',
  voting: '阁议中',
  convergence: '合案复议',
  completed: '已完成',
  failed: '失败',
  cancelled: '已取消',
}

export function seatStatus(phase: SessionPhase, seat: SeatKind, events: SessionEvent[] = []) {
  const failed = events.some((event) => event.event_type === 'seat_failed' && JSON.stringify(event.payload).includes(seat))
  if (failed) return '失败'
  if (phase === 'cancelled') return '已取消'
  if (phase === 'draft') return '待陈策'
  if (phase === 'independent_deliberation') return '陈策中'
  if (phase === 'cross_critique') return '批议中'
  if (phase === 'revision') return '已复议'
  if (phase === 'voting' || phase === 'convergence') return '已投策'
  if (phase === 'completed') return '已投策'
  return '失败'
}

export function hasMajority(details: SessionDetails) {
  return details.session.result?.status === 'majority_reached' || details.artifacts.decision?.status === 'majority_reached'
}

export function seatRunStats(runs: SeatRunTrace[]) {
  return (['mouyuan', 'jingshi', 'chizheng'] as SeatKind[]).map((seat) => {
    const scoped = runs.filter((run) => run.seat === seat)
    return {
      seat,
      calls: scoped.length,
      failed: scoped.filter((run) => run.status === 'failed').length,
      repaired: scoped.filter((run) => run.repair_attempted && run.status === 'completed').length,
      durationMs: scoped.reduce((sum, run) => sum + Number(run.duration_ms || 0), 0),
      tokens: scoped.reduce((sum, run) => sum + (run.total_tokens ?? 0), 0),
      promptVersions: Array.from(new Set(scoped.map((run) => run.prompt_version))).join(', '),
    }
  })
}
