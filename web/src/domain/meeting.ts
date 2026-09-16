export type MeetingStage = 'lobby' | 'initial' | 'cross_review' | 'synthesis' | 'completed' | 'paused'
export type ParticipantState = 'online' | 'stale' | 'left'
export type TurnStage = 'initial' | 'cross_review' | 'synthesis'
export type TurnStatus = 'pending' | 'claimed' | 'submitted' | 'skipped'
export type QuestionStatus = 'open' | 'deferred' | 'answered'

export interface MeetingSummary {
  meeting_id: string
  title: string
  goal: string
  stage: MeetingStage
  participant_limit: number
  participant_count: number
  participants_online: number
  created_at: string
  updated_at: string
  paused_reason: string | null
  has_final: boolean
}

export interface MeetingParticipant {
  id: string
  display_name: string
  client_name: string
  joined_at: string
  last_seen: string
  state: ParticipantState
}

export interface MeetingQuestion {
  id: string
  question: string
  reason: string
  missing_key: string
  asked_by_count: number
  status: QuestionStatus
  answer?: string | null
  answered_at?: string | null
}

export interface MeetingTurnView {
  id: string
  stage: TurnStage
  assignee: string
  assignee_name: string
  status: TurnStatus
  response: string | null
  skip_reason: string | null
  created_at: string
  updated_at: string
}

export interface StageProgress {
  total: number
  submitted: number
  skipped: number
}

export interface MeetingProgress {
  turns: Record<TurnStage, StageProgress>
  participants_online: number
  questions_open: number
  questions_deferred: number
}

export interface MeetingFinal {
  response: string
  synthesizer: string
  degraded: boolean
  skipped_turns: number
}

export interface MeetingDetail {
  meeting_id: string
  title: string
  goal: string
  context: string | null
  stage: MeetingStage
  participant_limit: number
  participants: MeetingParticipant[]
  active_question: MeetingQuestion | null
  progress: MeetingProgress
  paused_reason: string | null
  created_at: string
  updated_at: string
  final: MeetingFinal | null
  owner_access: boolean
  mcp_endpoint: string | null
  join_code: string | null
  questions: MeetingQuestion[]
  transcript: MeetingTurnView[]
}

export interface CreateMeetingInput {
  title: string
  goal: string
  context: string
  participant_limit: 2 | 3
}

export interface CreateMeetingResponse {
  meeting_id: string
  join_code: string
  owner_token: string
  participant_limit: number
  stage: MeetingStage
  mcp_endpoint: string | null
}
