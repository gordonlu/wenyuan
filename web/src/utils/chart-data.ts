import type { DonutSegment } from '../components/charts/DonutChart.vue'
import type { RadarAxis } from '../components/charts/RadarChart.vue'
import type { EvidenceSummary, DiscussionQualityMetrics } from '../domain/session'

const sourceColors: Record<string, string> = {
  internal: '#0f8aa1',
  web_search: '#17a8bd',
  file: '#0a6f83',
  code: '#5f6870',
  log: '#8b949e',
  data: '#b8c4cc',
}

const sourceLabels: Record<string, string> = {
  internal: '内部知识',
  web_search: '网络搜索',
  file: '文件',
  code: '代码',
  log: '日志',
  data: '数据',
}

export function evidenceDonutSegments(summary?: EvidenceSummary | null): DonutSegment[] {
  if (!summary || !summary.total) return []
  const entries = Object.entries(summary.by_source).sort((a, b) => b[1] - a[1])
  return entries.map(([kind, count]) => ({
    label: sourceLabels[kind] || kind,
    value: count,
    color: sourceColors[kind] || '#5f6870',
  }))
}

export function qualityRadarAxes(metrics?: DiscussionQualityMetrics | null): RadarAxis[] {
  if (!metrics) return []
  return [
    { label: '批议有效率', value: Math.round((metrics.critique_effectiveness_rate || 0) * 100), max: 100 },
    { label: '复议修改率', value: Math.round((metrics.revision_change_rate || 0) * 100), max: 100 },
    { label: '少数留议率', value: Math.round((metrics.minority_retention_rate || 0) * 100), max: 100 },
    { label: '票数集中度', value: Math.round((metrics.vote_concentration || 0) * 100), max: 100 },
    { label: '自投率', value: Math.round((metrics.self_vote_rate || 0) * 100), max: 100 },
  ]
}
