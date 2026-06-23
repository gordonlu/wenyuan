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

export type VoteStrategy = 'simple_majority' | 'risk_veto' | 'unanimous' | 'conditional_pass' | 'weighted_score'

export interface VotePolicy {
  allow_self_vote: boolean
  strategy: VoteStrategy
  min_score_threshold?: number
}

export interface Proposal {
  id: string
  proposed_by: SeatKind
  title: string
  summary: string
  source_idea_ids?: string[]
  adopted_points?: string[]
  rejected_points?: string[]
  rejection_reasons?: string[]
  changes_from_initial?: string[]
  user_value: string
  implementation_path: string
  risks: string[]
  success_metrics: string[]
  confidence?: number
}

export interface Decision {
  status: 'majority_reached' | 'conditionally_adopted' | 'no_majority'
  selected_proposal?: Proposal | null
  vote_count: number
  majority_reasons: string[]
  minority_opinion: string[]
  adoption_conditions: string[]
  unresolved_questions: string[]
  next_steps: string[]
  self_vote_count: number
  minority_choices?: Array<{
    seat: SeatKind
    proposal_id: string
    reason: string
    reassessment_condition: string
    has_risk_warning: boolean
  }>
  reassessment_conditions?: string[]
  has_risk_blocker?: boolean
}

export interface ScribeReport {
  consensus_summary: string
  structural_gaps: string[]
  unresolved_conflicts: string[]
  final_report: string
}

export interface SessionRecord {
  id: string
  title: string
  topic: string
  context: string
  mode: 'three_seat' | 'single_agent'
  phase: SessionPhase
  result?: Decision | null
  failure_reason?: string | null
  vote_policy?: VotePolicy | null
  scribe_enabled?: boolean
  search_enabled?: boolean
  external_evidence?: EvidenceItem[]
  external_tool_runs?: ToolRun[]
}

export interface SessionSummary {
  id: string
  title: string
  mode: 'three_seat' | 'single_agent'
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

export type IdeaStatus = 'proposed' | 'expanded' | 'challenged' | 'merged' | 'shortlisted' | 'rejected' | 'adopted'

export type Critique = {
  reviewer: SeatKind
  target_seat: SeatKind
  strongest_point?: string
  weakest_point?: string
  hidden_assumption?: string
  challenge: string
  counterexample?: string
  suggested_improvement: string
  evidence_question?: string
}

export type IdeaCard = {
  id: string
  proposed_by: SeatKind
  source_seats?: SeatKind[]
  title: string
  summary: string
  value: string
  mechanism?: string
  unconventional?: boolean
  assumptions?: string[]
  risks?: string[]
  status?: IdeaStatus
  challenged_by?: string[]
  referenced_by_proposals?: string[]
  merged_into?: string | null
}

export type Vote = {
  voter: SeatKind
  proposal_id: string
  final_choice: boolean
  reason: string
  key_evidence?: string
  blocking_issue?: string
}

export interface DiscussionArtifacts {
  ideas: IdeaCard[]
  critiques: Critique[]
  proposals: Proposal[]
  votes: Vote[]
  seat_runs: SeatRunTrace[]
  decision?: Decision | null
  quality?: DiscussionQualityMetrics
  claims?: Array<{
    id: string
    proposed_by: SeatKind
    content: string
    context: string
    is_supported: boolean
    evidence_ids?: string[]
    assessment_ids?: string[]
  }>
  evidence?: EvidenceItem[]
  assessments?: Array<{
    id: string
    assessor: SeatKind
    evidence_id: string
    claim_id: string
    supports_claim: boolean
    reasoning: string
    confidence: number
  }>
  claim_evidence_links?: Array<{
    claim_id: string
    evidence_id: string
    link_type: string
  }>
  events: string[]
  scribe_report?: ScribeReport | null
  topic_type?: string | null
  tool_runs?: ToolRun[]
}

export interface DiscussionQualityMetrics {
  idea_duplicate_rate: number
  seat_similarity: number
  high_similarity_detected?: boolean
  critique_effectiveness_rate: number
  revision_change_rate: number
  self_vote_rate: number
  vote_concentration: number
  minority_retention_rate: number
  average_tokens: number
  average_duration_ms: number
}

export interface RevisionDiff {
  seat: SeatKind
  ideaTitles: string[]
  proposalTitle?: string
  summaryChanged: boolean
  titleChanged: boolean
  adoptedIdeaCount: number
  initialSummary: string
  revisedSummary: string
  addedImplementationPath?: string
  addedSuccessMetrics: string[]
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
  seats?: SeatRecord[]
  execution: ExecutionInfo
  events: SessionEvent[]
}

export interface SourceSafetyFlags {
  prompt_injection_risk?: boolean
  contains_control_chars?: boolean
  truncated?: boolean
  warnings?: string[]
}

export interface EvidenceItem {
  id: string
  proposed_by: SeatKind
  kind: 'fact' | 'inference' | 'preference'
  content: string
  source: string
  source_fetched_at?: string | null
  source_hash?: string | null
  claim_ids?: string[]
  source_kind?: 'internal' | 'web_search' | 'file' | 'code' | 'log' | 'data'
  trust_level?: 'internal' | 'untrusted_external' | 'user_provided' | 'verified_external'
  safety_flags?: SourceSafetyFlags
}

export interface DocumentChunk {
  index: number
  locator: string
  text: string
  safety_flags: SourceSafetyFlags
}

export interface ParsedDocument {
  filename: string
  mime_type: string
  sha256: string
  chunks: DocumentChunk[]
}

export interface ParseDocumentResponse {
  document: ParsedDocument
  evidence: EvidenceItem[]
  tool_run: ToolRun
}

export interface CodeSearchMatch {
  path: string
  line_number: number
  line: string
  context_before: string[]
  context_after: string[]
}

export interface CodeSearchResultSet {
  query: string
  root: string
  matches: CodeSearchMatch[]
}

export interface CodeSearchResponse {
  result: CodeSearchResultSet
  evidence: EvidenceItem[]
  tool_run: ToolRun
}



export interface ToolRun {
  id: string
  seat?: SeatKind | null
  phase?: SessionPhase | null
  tool_name: string
  input_summary: string
  input_hash: string
  status: string
  duration_ms: number
  evidence_ids?: string[]
  error?: string | null
  created_at: string
}

export interface SeatRecord {
  session_id: string
  seat: SeatKind
  status: string
  last_error?: string | null
  system_prompt: string
  conversation: Array<{ role: string; content: string }>
  provider_ref: string
}

export interface ExecutionInfo {
  running: boolean
  lease_expires_at?: string | null
  recovery_state: 'idle' | 'running' | 'completed' | 'failed' | 'cancelled' | 'retry_required' | string
}

export interface ProviderSettings {
  provider: string
  base_url: string
  model: string
  api_key_configured: boolean
  api_key_hint?: string
}

export interface TestProviderResponse {
  ok: boolean
  latency_ms?: number
  message: string
  error_kind?: string
}

export interface ConfigStatus {
  provider_configured: boolean
  provider_kind: string
  model: string
  seat_models: Record<string, string>
  database_url: string
  version: string
  search_provider: string
  available_models: Array<{ value: string; label: string }>
  seat_available_models: Record<string, Array<{ value: string; label: string }>>
}

export interface UserPreferences {
  defaults: {
    mode: 'three_seat' | 'single_agent'
    scribe_enabled: boolean
    search_enabled: boolean
    vote_strategy: VoteStrategy
    allow_self_vote: boolean
    view_mode: 'workbench' | 'report' | string
  }
  models: {
    mouyuan: string
    jingshi: string
    chizheng: string
  }
  tools: {
    code_search_root: string
    max_file_size_mb: number
  }
}

export const seatLabels: Record<SeatKind, string> = {
  mouyuan: '谋远席',
  jingshi: '经世席',
  chizheng: '持正席',
}

export const ideaStatusLabels: Record<string, string> = {
  proposed: '已提出',
  expanded: '已扩展',
  challenged: '受质疑',
  merged: '已合并',
  shortlisted: '入围策案',
  rejected: '已拒绝',
  adopted: '已采纳',
}

export const evidenceKindLabels: Record<string, string> = {
  fact: '事实',
  inference: '推断',
  preference: '偏好',
}

export const evidenceSourceKindLabels: Record<string, string> = {
  internal: '内部推理',
  web_search: '网络搜索',
  file: '上传文件',
  code: '代码',
  log: '日志',
  data: '数据',
}

export const evidenceTrustLabels: Record<string, string> = {
  internal: '内部',
  untrusted_external: '不可信外部',
  user_provided: '用户提供',
  verified_external: '已验证外部',
}

export function evidenceSafetyLabels(flags?: SourceSafetyFlags) {
  const labels: string[] = []
  if (flags?.prompt_injection_risk) labels.push('疑似注入')
  if (flags?.contains_control_chars) labels.push('控制字符已净化')
  if (flags?.truncated) labels.push('已截断')
  for (const warning of flags?.warnings ?? []) {
    if (warning.trim()) labels.push(warning)
  }
  return labels
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

export const voteStrategyLabels: Record<VoteStrategy, string> = {
  simple_majority: '普通多数（2/3）',
  risk_veto: '风险否决',
  unanimous: '全票通过（3/3）',
  conditional_pass: '有条件通过',
  weighted_score: '加权评分',
}

export const voteStrategyDescriptions: Record<VoteStrategy, string> = {
  simple_majority: '两席以上同意即形成多数',
  risk_veto: '任何一席提出阻塞问题即否决',
  unanimous: '需要三席全部同意',
  conditional_pass: '同普通多数，但增加持续监控条件',
  weighted_score: '按五项评分加权总分决定',
}

export const modeLabels: Record<string, string> = {
  three_seat: '三席合议',
  single_agent: '单 Agent',
}

export function seatStatus(phase: SessionPhase, seat: SeatKind, events: SessionEvent[] = [], running = false) {
  const latest = [...events]
    .reverse()
    .find((event) => ['seat_started', 'seat_completed', 'seat_failed'].includes(event.event_type) && eventPayloadIncludesSeat(event.payload, seat))
  if (latest?.event_type === 'seat_failed') return '失败'
  if (latest?.event_type === 'seat_started') return running ? '思考中' : '进行中'
  if (latest?.event_type === 'seat_completed') return '已完成'
  if (phase === 'cancelled') return '已取消'
  if (phase === 'draft') return '待陈策'
  if (running && ['independent_deliberation', 'cross_critique', 'revision', 'voting', 'convergence'].includes(phase)) return '等待'
  if (phase === 'independent_deliberation') return '陈策中'
  if (phase === 'cross_critique') return '批议中'
  if (phase === 'revision') return '已复议'
  if (phase === 'voting' || phase === 'convergence') return '已投策'
  if (phase === 'completed') return '已投策'
  return '失败'
}

export function seatStatusClass(phase: SessionPhase, seat: SeatKind, events: SessionEvent[] = [], running = false) {
  const status = seatStatus(phase, seat, events, running)
  if (status === '思考中') return 'active'
  if (status === '等待' || status.endsWith('中')) return 'pending'
  if (status === '已完成' || status === '已复议' || status === '已投策') return 'ok'
  if (status === '失败' || status === '已取消') return 'danger'
  return ''
}

function eventPayloadIncludesSeat(payload: unknown, seat: SeatKind) {
  return typeof payload === 'object' && payload !== null && 'seat' in payload && (payload as { seat?: unknown }).seat === seat
}

export function hasMajority(details: SessionDetails) {
  const status = details.session.result?.status ?? details.artifacts.decision?.status
  return status === 'majority_reached' || status === 'conditionally_adopted'
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
      hasUsage: scoped.some((run) => typeof run.total_tokens === 'number'),
      promptVersions: Array.from(new Set(scoped.map((run) => run.prompt_version))).join(', '),
    }
  })
}

export function revisionDiffs(details: SessionDetails): RevisionDiff[] {
  return (['mouyuan', 'jingshi', 'chizheng'] as SeatKind[]).map((seat) => {
    const seatIdeas = details.artifacts.ideas.filter((idea) => idea.proposed_by === seat)
    const proposal = details.artifacts.proposals.find((item) => item.proposed_by === seat)
    const referencedIdeas = proposal?.source_idea_ids?.length
      ? seatIdeas.filter((idea) => proposal.source_idea_ids?.includes(idea.id))
      : seatIdeas
    const initialSummary = referencedIdeas.map((idea) => idea.summary).filter(Boolean).join('\n') || seatIdeas.map((idea) => idea.summary).filter(Boolean).join('\n')
    const initialTitle = referencedIdeas.map((idea) => idea.title).filter(Boolean).join(' / ') || seatIdeas.map((idea) => idea.title).filter(Boolean).join(' / ')
    const revisedSummary = proposal?.summary ?? ''
    const proposalTitle = proposal?.title

    return {
      seat,
      ideaTitles: referencedIdeas.map((idea) => idea.title),
      proposalTitle,
      summaryChanged: normalizeText(initialSummary) !== normalizeText(revisedSummary) || Boolean(proposal?.changes_from_initial?.length),
      titleChanged: normalizeText(initialTitle) !== normalizeText(proposalTitle ?? ''),
      adoptedIdeaCount: referencedIdeas.length,
      initialSummary,
      revisedSummary,
      addedImplementationPath: proposal?.implementation_path,
      addedSuccessMetrics: proposal?.success_metrics ?? [],
    }
  })
}

export function qualityMetricRows(metrics?: DiscussionQualityMetrics, hasUsage = true) {
  const safe = metrics ?? {
    idea_duplicate_rate: 0,
    seat_similarity: 0,
    high_similarity_detected: false,
    critique_effectiveness_rate: 0,
    revision_change_rate: 0,
    self_vote_rate: 0,
    vote_concentration: 0,
    minority_retention_rate: 0,
    average_tokens: 0,
    average_duration_ms: 0,
  }
  return [
    { label: 'Idea 重复率', value: formatPercent(safe.idea_duplicate_rate) },
    { label: '三席相似度', value: formatPercent(safe.seat_similarity) },
    { label: '高度重复检测', value: safe.high_similarity_detected ? '是' : '否' },
    { label: '批议有效率', value: formatPercent(safe.critique_effectiveness_rate) },
    { label: '复议修改率', value: formatPercent(safe.revision_change_rate) },
    { label: '自投率', value: formatPercent(safe.self_vote_rate) },
    { label: '票数集中度', value: formatPercent(safe.vote_concentration) },
    { label: '少数留议率', value: formatPercent(safe.minority_retention_rate) },
    { label: '平均 token', value: hasUsage ? String(Math.round(safe.average_tokens)) : '暂无' },
    { label: '平均耗时', value: `${(safe.average_duration_ms / 1000).toFixed(1)} 秒` },
  ]
}

export function exportSessionMarkdown(details: SessionDetails, level: 'brief' | 'standard' | 'audit' = 'brief') {
  const decision = details.session.result ?? details.artifacts.decision
  const hasUsage = details.artifacts.seat_runs.some((run) => typeof run.total_tokens === 'number')
  const lines: string[] = [
    `# ${markdownText(details.session.title) || '未命名议题'}`,
    '',
    `- 阶段：${phaseLabels[details.session.phase]}`,
    `- 模式：${modeLabels[details.session.mode]}`,
    `- Session：${details.session.id}`,
    '',
  ]

  if (level === 'brief') {
    lines.push('## 结论摘要', '')
    if (decision) {
      appendDecisionFields(lines, decision)
      appendList(lines, '关键理由', decision.majority_reasons.slice(0, 3))
      appendList(lines, '下一步', decision.next_steps.slice(0, 3))
    } else {
      lines.push('暂无表决结果。')
    }

    lines.push('', '## 议题', '')
    appendParagraph(lines, details.session.topic)

    const selectedProposal = decision?.selected_proposal ?? details.artifacts.proposals[0]
    if (selectedProposal) {
      lines.push('## 推荐方案', '', `### ${seatLabels[selectedProposal.proposed_by]}：${markdownText(selectedProposal.title)}`)
      appendParagraph(lines, selectedProposal.summary)
      appendField(lines, '落地路径', selectedProposal.implementation_path)
      appendList(lines, '主要风险', selectedProposal.risks.slice(0, 3))
    }

    if (decision?.minority_choices?.length || decision?.minority_opinion.length || decision?.adoption_conditions.length || decision?.unresolved_questions.length) {
      lines.push('## 风险与留议', '')
      appendMinority(lines, decision)
      appendList(lines, '采纳条件', decision.adoption_conditions.slice(0, 3))
      appendList(lines, '未决问题', decision.unresolved_questions.slice(0, 3))
    }

    lines.push('## 说明', '', '普通报告仅保留结论和行动要点。需要完整推演、证据、工具轨迹和质量指标时，请导出深度研究报告。')
    return `${lines.join('\n').replace(/\n{3,}/g, '\n\n')}\n`
  }

  lines.push('## 1. 结论', '')
  if (decision) {
    appendDecisionFields(lines, decision)
    appendList(lines, '多数理由', decision.majority_reasons)
    appendList(lines, '下一步', decision.next_steps)
    appendList(lines, '采纳条件', decision.adoption_conditions)
    appendList(lines, '未决问题', decision.unresolved_questions)
  } else {
    lines.push('暂无表决结果。')
  }

  lines.push('', '## 2. 议题与背景', '')
  appendParagraph(lines, details.session.topic)
  appendBlock(lines, '背景', details.session.context)

  lines.push('', '## 3. 策案对比', '')
  for (const proposal of details.artifacts.proposals) {
    lines.push(`### ${seatLabels[proposal.proposed_by]}：${markdownText(proposal.title)}`)
    appendParagraph(lines, proposal.summary)
    appendField(lines, '用户价值', proposal.user_value)
    appendField(lines, '落地路径', proposal.implementation_path)
    appendList(lines, '采纳观点', proposal.adopted_points)
    appendList(lines, '拒绝观点', proposal.rejected_points)
    appendList(lines, '拒绝理由', proposal.rejection_reasons)
    appendList(lines, '相较独议修改', proposal.changes_from_initial)
    appendList(lines, '风险', proposal.risks)
    appendList(lines, '成功指标', proposal.success_metrics)
    if (proposal.confidence !== undefined) lines.push(`- 置信度：${formatPercent(proposal.confidence)}`)
    lines.push('')
  }

  lines.push('## 4. 三席推演', '', '### 独议', '')
  for (const idea of details.artifacts.ideas) {
    lines.push(`#### ${seatLabels[idea.proposed_by]}：${markdownText(idea.title)}`)
    appendParagraph(lines, idea.summary)
    appendField(lines, '价值', idea.value)
    appendField(lines, '机制', idea.mechanism)
    if (idea.unconventional) lines.push('- 非主流方向：是')
    appendList(lines, '假设', idea.assumptions)
    appendList(lines, '风险', idea.risks)
    lines.push('')
  }

  lines.push('### 批议摘要', '')
  for (const critique of details.artifacts.critiques) {
    lines.push(`#### ${seatLabels[critique.reviewer]} → ${seatLabels[critique.target_seat]}`)
    appendField(lines, '最强点', critique.strongest_point)
    appendField(lines, '最弱点', critique.weakest_point)
    appendField(lines, '隐含假设', critique.hidden_assumption)
    appendField(lines, '挑战', critique.challenge)
    appendField(lines, '反例或失败条件', critique.counterexample)
    appendField(lines, '改进建议', critique.suggested_improvement)
    appendField(lines, '需要补证', critique.evidence_question)
    lines.push('')
  }

  lines.push('## 5. 表决与留议', '')
  if (decision) {
    appendDecisionFields(lines, decision)
    appendList(lines, '多数理由', decision.majority_reasons)
    appendMinority(lines, decision)
    appendList(lines, '重新评估条件', decision.reassessment_conditions)
    appendList(lines, '采纳条件', decision.adoption_conditions)
    appendList(lines, '未决问题', decision.unresolved_questions)
    appendList(lines, '下一步', decision.next_steps)
  } else {
    lines.push('暂无表决结果。')
  }

  if (details.artifacts.votes.length > 0) {
    lines.push('', '### 逐席投票', '')
    for (const vote of details.artifacts.votes) {
      const proposal = details.artifacts.proposals.find((item) => item.id === vote.proposal_id)
      lines.push(`- ${seatLabels[vote.voter]}：${vote.final_choice ? '支持' : '不支持'}${proposal ? ` ${markdownText(proposal.title)}` : ''}。${markdownText(vote.reason)}`)
    }
  }

  if (details.artifacts.evidence?.length || details.artifacts.tool_runs?.length) {
    lines.push('', '## 6. 证据与工具', '')
    if (details.artifacts.evidence?.length) {
      for (const ev of details.artifacts.evidence.slice(0, level === 'audit' ? undefined : 12)) {
        const sourceKind = evidenceSourceKindLabels[ev.source_kind ?? 'internal'] ?? ev.source_kind ?? '未知来源'
        const trust = evidenceTrustLabels[ev.trust_level ?? 'internal'] ?? ev.trust_level ?? '未知信任级别'
        const content = markdownText(ev.content)
        if (!content) continue
        lines.push(`- [${evidenceKindLabels[ev.kind]}｜${sourceKind}｜${trust}] ${content}${ev.source ? ` (${compactReportSource(ev.source)})` : ''}`)
      }
      lines.push('')
    }
    if (details.artifacts.tool_runs?.length) {
      appendToolRuns(lines, details.artifacts.tool_runs, level === 'audit')
    }
  }

  lines.push('', '## 7. 讨论质量指标', '')
  for (const metric of qualityMetricRows(details.artifacts.quality, hasUsage)) {
    lines.push(`- ${metric.label}：${metric.value}`)
  }

  if (level === 'audit') {
    if (details.artifacts.claims?.length) {
      lines.push('', '## 主张池（Audit）', '')
      for (const claim of details.artifacts.claims) {
        lines.push(`- ${seatLabels[claim.proposed_by as SeatKind] || claim.proposed_by}：${markdownText(claim.content)}${claim.is_supported ? ' ✅' : ' ❌'}`)
      }
    }
    if (details.artifacts.assessments?.length) {
      lines.push('', '## 评估记录（Audit）', '')
      for (const a of details.artifacts.assessments) {
        lines.push(`- ${a.assessor} → ${a.claim_id}：${a.supports_claim ? '支持' : '反对'}`)
      }
    }
    if (details.artifacts.seat_runs.length) {
      lines.push('', '## 执行轨迹（Audit）', '')
      for (const run of details.artifacts.seat_runs) {
        lines.push(`- ${seatLabels[run.seat]} [${run.phase}]：${run.status}`, '')
        if (run.raw_output) {
          lines.push('  ```', `  ${run.raw_output.slice(0, 500).replace(/\n/g, '\n  ')}`, '  ```', '')
        }
      }
    }
  }

  return `${lines.join('\n').replace(/\n{3,}/g, '\n\n')}\n`
}

function decisionLabel(decision: Decision) {
  if (decision.status === 'majority_reached') return '形成多数'
  if (decision.status === 'conditionally_adopted') return '有条件通过'
  return '未形成多数'
}

function appendList(lines: string[], label: string, values?: string[]) {
  const cleaned = cleanReportList(values)
  if (cleaned.length === 0) return
  lines.push(`- ${label}：`)
  for (const value of cleaned) {
    lines.push(`  - ${value}`)
  }
}

function appendParagraph(lines: string[], value?: string | null) {
  const text = markdownText(value)
  if (text) lines.push(text, '')
}

function appendBlock(lines: string[], label: string, value?: string | null) {
  const text = markdownText(value)
  if (!text) return
  lines.push(`### ${label}`, '', text, '')
}

function appendField(lines: string[], label: string, value?: string | null) {
  const text = markdownText(value)
  if (text) lines.push(`- ${label}：${text}`)
}

function appendDecisionFields(lines: string[], decision: Decision) {
  lines.push(`- 状态：${decisionLabel(decision)}`)
  if (decision.selected_proposal) {
    lines.push(`- 多数策案：${seatLabels[decision.selected_proposal.proposed_by]} - ${markdownText(decision.selected_proposal.title)}`)
  }
  lines.push(`- 有效票数：${decision.vote_count}`)
  lines.push(`- 自投数：${decision.self_vote_count}`)
  if (decision.has_risk_blocker) lines.push('- 风险阻塞：存在')
  lines.push('')
}

function appendMinority(lines: string[], decision: Decision) {
  const choices = decision.minority_choices ?? []
  if (choices.length) {
    lines.push('- 少数留议：')
    for (const choice of choices) {
      const reason = markdownText(choice.reason)
      const reassessment = markdownText(choice.reassessment_condition)
      if (reason) lines.push(`  - ${seatLabels[choice.seat]}：${reason}${choice.has_risk_warning ? '（含风险提醒）' : ''}`)
      if (reassessment) lines.push(`    - 重新评估：${reassessment}`)
    }
    return
  }
  appendList(lines, '少数留议', decision.minority_opinion)
}

function appendToolRuns(lines: string[], runs: ToolRun[], includeErrors = false) {
  const visibleRuns = includeErrors ? runs : runs.slice(0, 12)
  for (const run of visibleRuns) {
    const seat = run.seat ? `${seatLabels[run.seat]} · ` : ''
    const phase = run.phase ? `${phaseLabels[run.phase]} · ` : ''
    lines.push(`- ${seat}${phase}${toolNameLabel(run.tool_name)}：${run.status}，${(run.duration_ms / 1000).toFixed(1)} 秒，${run.evidence_ids?.length ?? 0} 条证据`)
    appendField(lines, '输入', run.input_summary)
    if (includeErrors) appendField(lines, '错误', run.error)
  }
  if (!includeErrors && runs.length > visibleRuns.length) {
    lines.push(`- 其余 ${runs.length - visibleRuns.length} 次工具调用已省略，可导出审计全文查看。`)
  }
  lines.push('')
}

export function cleanReportText(value?: string | null) {
  if (typeof value !== 'string') return ''
  let text = value
    .replace(/\r\n/g, '\n')
    .replace(/\u0000/g, '')
    .replace(/\{\{\s*[^{}]{1,80}\s*\}\}/g, '')
    .replace(/\[\s*(?:TODO|TBD|PLACEHOLDER|待补充|占位符)[^\]]{0,80}\]/gi, '')
    .trim()

  if (!text) return ''
  text = stripWrappingFence(text)

  const lines = text
    .split('\n')
    .map((line) => line.replace(/[ \t]+$/g, '').trim())
    .filter((line) => !isPlaceholderText(line))
    .filter((line) => !/^(?:output_schema|json_schema|schema|fields?|params?|parameters?|placeholder|字段|参数|占位符)\s*[:：]?\s*$/i.test(line))
    .map(stripSchemaPrefix)
    .filter(Boolean)

  return lines.join('\n').replace(/\n{3,}/g, '\n\n').trim()
}

function cleanReportList(values?: string[]) {
  return values?.map((value) => markdownText(value)).filter(Boolean) ?? []
}

function markdownText(value?: string | null) {
  return cleanReportText(value)
}

function stripWrappingFence(value: string) {
  return value
    .replace(/^```(?:json|jsonc|markdown|md|text)?\s*\n/i, '')
    .replace(/\n```\s*$/i, '')
    .trim()
}

function stripSchemaPrefix(line: string) {
  return line.replace(/^[-*]?\s*(?:title|summary|value|mechanism|user_value|implementation_path|risk|risks|success_metrics|confidence|majority_reasons?|minority_opinion|next_steps?|adoption_conditions?|unresolved_questions?|reassessment_condition|reason|challenge|counterexample|suggested_improvement|evidence_question|final_report)\s*[:：]\s*/i, '')
}

function isPlaceholderText(value: string) {
  const normalized = value.replace(/[。.,，\s]+$/g, '').trim().toLowerCase()
  return [
    '',
    '-',
    '--',
    'n/a',
    'na',
    'none',
    'null',
    'undefined',
    'todo',
    'tbd',
    'placeholder',
    '待补充',
    '暂无',
    '无',
    '无明确',
    '未提供',
    '不适用',
  ].includes(normalized)
}

function compactReportSource(source: string) {
  const cleaned = cleanReportText(source)
  if (!cleaned) return ''
  try {
    const url = new URL(cleaned)
    return `${url.hostname}${url.pathname === '/' ? '' : url.pathname}`
  } catch {
    return cleaned.length > 80 ? `${cleaned.slice(0, 77)}...` : cleaned
  }
}

export function toolRunSummary(runs: ToolRun[]) {
  const total = runs.length
  const completed = runs.filter((r) => r.status === 'completed').length
  const failed = runs.filter((r) => r.status !== 'completed').length
  const by_tool: Record<string, number> = {}
  for (const r of runs) {
    by_tool[r.tool_name] = (by_tool[r.tool_name] ?? 0) + 1
  }
  const total_ms = runs.reduce((s, r) => s + r.duration_ms, 0)
  return { total, completed, failed, by_tool, total_ms }
}

export function toolNameLabel(name: string): string {
  const labels: Record<string, string> = {
    web_search: '网页搜索',
    document_parse: '文档解析',
    code_search: '代码搜索',
  }
  return labels[name] ?? name
}

export interface EvidenceSummary {
  total: number
  by_source: Record<string, number>
  untrusted_count: number
  injection_risk_count: number
  unverified_claims: number
  has_safety_warnings: boolean
}

export function evidenceSummary(details: SessionDetails): EvidenceSummary {
  const evidence = details.artifacts.evidence ?? []
  const claims = details.artifacts.claims ?? []
  const by_source: Record<string, number> = {}
  let untrusted_count = 0
  let injection_risk_count = 0

  for (const ev of evidence) {
    const kind = ev.source_kind ?? 'internal'
    by_source[kind] = (by_source[kind] ?? 0) + 1
    if (ev.trust_level === 'untrusted_external') untrusted_count++
    if (ev.safety_flags?.prompt_injection_risk) injection_risk_count++
  }

  return {
    total: evidence.length,
    by_source,
    untrusted_count,
    injection_risk_count,
    unverified_claims: claims.filter((c) => !c.is_supported).length,
    has_safety_warnings: evidence.some((ev) => {
      const f = ev.safety_flags
      return f?.prompt_injection_risk || f?.contains_control_chars || !!f?.warnings?.length
    }),
  }
}

export interface DecisionDigest {
  has_decision: boolean
  status_label: string
  status_class: 'ok' | 'warn' | 'danger'
  selected_proposal_title: string
  selected_proposal_seat: SeatKind | ''
  vote_count: number
  has_risk_blocker: boolean
  minority_count: number
  majority_reason_summary: string
  minority_summary: string
  next_step_summary: string
  evidence_total: number
  has_unverified_evidence: boolean
  unverified_claims: number
  has_untrusted_external: boolean
  has_injection_risk: boolean
}

export function decisionDigest(details: SessionDetails, evidenceSummaryResult?: EvidenceSummary): DecisionDigest {
  const decision = details.session.result ?? details.artifacts.decision
  const evSum = evidenceSummaryResult ?? evidenceSummary(details)

  if (!decision) {
    return {
      has_decision: false,
      status_label: '尚无结论',
      status_class: 'warn',
      selected_proposal_title: '',
      selected_proposal_seat: '',
      vote_count: 0,
      has_risk_blocker: false,
      minority_count: 0,
      majority_reason_summary: '',
      minority_summary: '',
      next_step_summary: '',
      evidence_total: evSum.total,
      has_unverified_evidence: evSum.unverified_claims > 0,
      unverified_claims: evSum.unverified_claims,
      has_untrusted_external: evSum.untrusted_count > 0,
      has_injection_risk: evSum.injection_risk_count > 0,
    }
  }

  const status_label = decision.status === 'majority_reached'
    ? '形成多数'
    : decision.status === 'conditionally_adopted'
      ? '有条件通过'
      : '未形成多数'

  const status_class: 'ok' | 'warn' | 'danger' = decision.status === 'no_majority'
    ? 'danger'
    : decision.has_risk_blocker
      ? 'warn'
      : 'ok'

  return {
    has_decision: true,
    status_label,
    status_class,
    selected_proposal_title: decision.selected_proposal?.title ?? '',
    selected_proposal_seat: decision.selected_proposal?.proposed_by ?? '',
    vote_count: decision.vote_count,
    has_risk_blocker: !!decision.has_risk_blocker,
    minority_count: decision.minority_choices?.length ?? 0,
    majority_reason_summary: decision.majority_reasons?.slice(0, 2).join('；') ?? '',
    minority_summary: decision.minority_opinion?.slice(0, 2).join('；') ?? '',
    next_step_summary: decision.next_steps?.slice(0, 2).join('；') ?? '',
    evidence_total: evSum.total,
    has_unverified_evidence: evSum.unverified_claims > 0,
    unverified_claims: evSum.unverified_claims,
    has_untrusted_external: evSum.untrusted_count > 0,
    has_injection_risk: evSum.injection_risk_count > 0,
  }
}

function normalizeText(value: string) {
  return value.replace(/\s+/g, '').toLowerCase()
}

function formatPercent(value: number) {
  return `${Math.round(value * 100)}%`
}
