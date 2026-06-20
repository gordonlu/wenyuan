<template>
  <Teleport to="body">
    <div v-if="visible" class="share-overlay" @click.self="$emit('close')">
      <div class="share-panel" role="dialog" aria-label="分享审议结果">
        <header class="share-head">
          <h2>分享</h2>
          <button class="icon" title="关闭" @click="$emit('close')"><X :size="18" /></button>
        </header>

        <div class="share-platform-tabs">
          <button
            v-for="p in platforms"
            :key="p.id"
            :class="['share-tab', { active: platform === p.id }]"
            @click="platform = p.id"
          >{{ p.label }}</button>
        </div>

        <div class="share-card-preview" ref="cardRef">
          <div v-if="platform === 'x'" class="share-card share-card-x">
            <div class="share-card-header">
              <span class="share-card-brand">文渊阁 Wenyuan</span>
              <span class="share-card-badge">AI 合议</span>
            </div>
            <div class="share-card-title">{{ digest.title }}</div>
            <div class="share-card-status">{{ digest.status_label }}</div>
            <div v-if="digest.selected_proposal_title" class="share-card-row">
              <span class="share-card-label">结论</span>
              <span class="share-card-value">{{ digest.selected_proposal_title }}</span>
            </div>
            <div v-if="digest.majority_summary" class="share-card-row">
              <span class="share-card-label">依据</span>
              <span class="share-card-value">{{ digest.majority_summary }}</span>
            </div>
            <div v-if="digest.risk_summary" class="share-card-row share-card-risk">
              <span class="share-card-label">风险</span>
              <span class="share-card-value">{{ digest.risk_summary }}</span>
            </div>
            <div class="share-card-footer">
              <span>{{ digest.evidence_total }} 项来源</span>
              <span>{{ digest.vote_count }} 票</span>
              <span>{{ digest.seat_count }} 席</span>
            </div>
          </div>

          <div v-if="platform === 'xiaohongshu'" class="share-card share-card-xhs">
            <div class="xhs-cover">
              <div class="xhs-cover-title">{{ digest.title }}</div>
              <div class="xhs-cover-badge">{{ digest.status_label }}</div>
            </div>
            <div class="xhs-body">
              <div class="xhs-section">
                <div class="xhs-section-label">三席观点</div>
                <div class="xhs-section-value">{{ seatSummary }}</div>
              </div>
              <div class="xhs-section">
                <div class="xhs-section-label">关键证据</div>
                <div class="xhs-section-value">{{ digest.evidence_total }} 项来源 · {{ digest.untrusted_count }} 项不可信</div>
              </div>
              <div v-if="digest.risk_summary" class="xhs-section xhs-risk">
                <div class="xhs-section-label">风险提示</div>
                <div class="xhs-section-value">{{ digest.risk_summary }}</div>
              </div>
            </div>
            <div class="xhs-footer">生成于 文渊阁 Wenyuan</div>
          </div>
        </div>

        <div class="share-actions">
          <button class="primary" @click="copyText">
            <Copy :size="16" />
            复制{{ platform === 'x' ? '推文' : '笔记' }}
          </button>
          <button class="primary" @click="downloadImage">
            <Download :size="16" />
            下载 PNG
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { Copy, Download, X } from '@lucide/vue'

interface ShareDigest {
  title: string
  status_label: string
  status_class: string
  selected_proposal_title: string
  majority_summary: string
  risk_summary: string
  evidence_total: number
  untrusted_count: number
  vote_count: number
  seat_count: number
}

const props = defineProps<{
  visible: boolean
  digest: ShareDigest
  title: string
  seatSummary: string
  evidenceTotal: number
  untrustedCount: number
  voteCount: number
}>()

defineEmits<{
  close: []
}>()

const platforms = [
  { id: 'x', label: 'X (Twitter)' },
  { id: 'xiaohongshu', label: '小红书' },
] as const

type PlatformId = (typeof platforms)[number]['id']
const platform = ref<PlatformId>('x')
const cardRef = ref<HTMLElement | null>(null)

function copyText() {
  const lines: string[] = []
  if (platform.value === 'x') {
    lines.push(`【合议结果】${props.digest.title}`)
    lines.push('')
    lines.push(`结论：${props.digest.status_label}`)
    if (props.digest.selected_proposal_title) lines.push(`多数策案：${props.digest.selected_proposal_title}`)
    if (props.digest.majority_summary) lines.push(`主要依据：${props.digest.majority_summary}`)
    if (props.digest.risk_summary) lines.push(`风险提醒：${props.digest.risk_summary}`)
    lines.push('')
    lines.push(`${props.digest.evidence_total} 项来源 · ${props.digest.vote_count} 票 · ${props.digest.seat_count} 席`)
    lines.push('')
    lines.push('#文渊阁 #AI合议')
  } else {
    lines.push(`标题：${props.digest.title}`)
    lines.push('')
    lines.push(`让三个 AI 席位对一个复杂议题分别思考、互相批议、修订方案，最终投票形成结论。`)
    lines.push('')
    if (props.digest.selected_proposal_title) lines.push(`最终判断：${props.digest.selected_proposal_title}`)
    lines.push('')
    lines.push(`三席观点：${props.seatSummary}`)
    lines.push('')
    lines.push(`来源证据：${props.digest.evidence_total} 项`)
    if (props.digest.risk_summary) lines.push(`风险提示：${props.digest.risk_summary}`)
    lines.push('')
    lines.push('#文渊阁 #AI合议 #决策工具')
  }
  navigator.clipboard.writeText(lines.join('\n'))
}

async function downloadImage() {
  const canvas = document.createElement('canvas')
  const width = platform.value === 'x' ? 1200 : 1080
  const height = platform.value === 'x' ? 675 : 1440
  canvas.width = width
  canvas.height = height
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  // Background
  const gradient = ctx.createLinearGradient(0, 0, 0, height)
  gradient.addColorStop(0, '#0f1a24')
  gradient.addColorStop(1, '#081119')
  ctx.fillStyle = gradient
  ctx.fillRect(0, 0, width, height)

  // Decorative accent line
  ctx.fillStyle = '#0f8aa1'
  ctx.fillRect(0, 0, width, 4)

  // Brand
  ctx.fillStyle = '#5f8494'
  ctx.font = '14px sans-serif'
  ctx.fillText('文渊阁 Wenyuan', 40, 50)

  // Title
  ctx.fillStyle = '#f0f4f8'
  ctx.font = 'bold 28px sans-serif'
  wrapText(ctx, props.digest.title, 40, 100, width - 80, 36)

  // Separator
  ctx.fillStyle = '#1f3640'
  ctx.fillRect(40, 160, 60, 3)

  // Status badge
  const isOk = props.digest.status_class === 'ok'
  ctx.fillStyle = isOk ? '#1a7a5c' : '#8a6a20'
  const statusText = props.digest.status_label
  ctx.font = '14px sans-serif'
  const statusW = ctx.measureText(statusText).width
  roundRect(ctx, 40, 180, statusW + 24, 28, 4)
  ctx.fillStyle = '#f0f4f8'
  ctx.fillText(statusText, 52, 199)

  // Evidence + votes row
  let yPos = 240
  ctx.fillStyle = '#8aa8b8'
  ctx.font = '16px sans-serif'
  ctx.fillText(`${props.digest.evidence_total} 项来源`, 40, yPos)
  ctx.fillText(`${props.digest.vote_count} 票`, 180, yPos)

  // Majority proposal
  yPos += 40
  if (props.digest.selected_proposal_title) {
    ctx.fillStyle = '#c0d4de'
    ctx.font = 'bold 18px sans-serif'
    ctx.fillText('多数策案', 40, yPos)
    yPos += 28
    ctx.fillStyle = '#f0f4f8'
    ctx.font = '20px sans-serif'
    const proposalLines = wrapText(ctx, props.digest.selected_proposal_title, 40, yPos, width - 80, 26)
    yPos += proposalLines * 28
  }

  // Risk
  if (props.digest.risk_summary) {
    yPos += 16
    ctx.fillStyle = '#b06040'
    ctx.font = 'bold 16px sans-serif'
    ctx.fillText('风险提醒', 40, yPos)
    yPos += 24
    ctx.fillStyle = '#d4a090'
    ctx.font = '16px sans-serif'
    const riskLines = wrapText(ctx, props.digest.risk_summary, 40, yPos, width - 80, 22)
    yPos += riskLines * 24
  }

  // Footer
  ctx.fillStyle = '#3a5564'
  ctx.font = '12px sans-serif'
  ctx.fillText('生成于 文渊阁 Wenyuan · wenyuan.dev', 40, height - 30)

  // Convert to blob and download
  canvas.toBlob((blob) => {
    if (!blob) return
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `wenyuan-${platform.value}-${Date.now()}.png`
    a.click()
    URL.revokeObjectURL(url)
  }, 'image/png')
}

function wrapText(ctx: CanvasRenderingContext2D, text: string, x: number, y: number, maxW: number, lineH: number): number {
  if (!text) return 0
  const chars = text.split('')
  let line = ''
  let lines = 0
  for (const ch of chars) {
    const test = line + ch
    if (ctx.measureText(test).width > maxW && line) {
      ctx.fillText(line, x, y)
      y += lineH
      lines++
      line = ch
    } else {
      line = test
    }
  }
  if (line) {
    ctx.fillText(line, x, y)
    lines++
  }
  return lines
}

function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  ctx.beginPath()
  ctx.moveTo(x + r, y)
  ctx.lineTo(x + w - r, y)
  ctx.quadraticCurveTo(x + w, y, x + w, y + r)
  ctx.lineTo(x + w, y + h - r)
  ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h)
  ctx.lineTo(x + r, y + h)
  ctx.quadraticCurveTo(x, y + h, x, y + h - r)
  ctx.lineTo(x, y + r)
  ctx.quadraticCurveTo(x, y, x + r, y)
  ctx.closePath()
  ctx.fill()
}
</script>

<style scoped>
.share-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(4px);
}

.share-panel {
  width: min(640px, 92vw);
  max-height: 90vh;
  overflow-y: auto;
  background: var(--color-surface);
  border-radius: var(--radius-md);
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.3);
}

.share-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--color-border-light);
}

.share-head h2 {
  margin: 0;
  font-size: 16px;
}

.share-platform-tabs {
  display: flex;
  gap: 4px;
  padding: 12px 20px 0;
}

.share-tab {
  padding: 6px 14px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm) var(--radius-sm) 0 0;
  background: transparent;
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-muted);
  cursor: pointer;
}

.share-tab.active {
  border-color: var(--color-accent);
  background: var(--color-accent-light);
  color: var(--color-accent-text);
}

.share-card-preview {
  padding: 20px;
}

.share-card {
  border-radius: var(--radius-md);
  overflow: hidden;
}

.share-card-x {
  background: linear-gradient(145deg, #0f1a24, #081119);
  color: #f0f4f8;
  padding: 32px;
}

.share-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.share-card-brand {
  font-size: 13px;
  font-weight: 700;
  color: #5f8494;
}

.share-card-badge {
  padding: 3px 8px;
  border-radius: 4px;
  background: rgba(15, 138, 161, 0.2);
  color: #8ddbd1;
  font-size: 11px;
  font-weight: 600;
}

.share-card-title {
  font-size: 22px;
  font-weight: 800;
  line-height: 1.3;
  margin-bottom: 12px;
}

.share-card-status {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 4px;
  font-size: 13px;
  font-weight: 700;
  margin-bottom: 20px;
}

.share-card-x .share-card-status {
  background: #1a7a5c;
  color: #ffffff;
}

.share-card-row {
  display: flex;
  gap: 12px;
  padding: 10px 0;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.share-card-label {
  min-width: 48px;
  font-size: 12px;
  font-weight: 700;
  color: #5f8494;
}

.share-card-value {
  font-size: 14px;
  line-height: 1.4;
  color: #c0d4de;
}

.share-card-risk .share-card-value {
  color: #d4a090;
}

.share-card-footer {
  display: flex;
  gap: 16px;
  margin-top: 16px;
  padding-top: 14px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  font-size: 12px;
  color: #5f8494;
}

/* Xiaohongshu card */
.share-card-xhs {
  background: #fffdf8;
  border: 1px solid #e5ddd0;
}

.xhs-cover {
  padding: 40px 28px 28px;
  background: linear-gradient(135deg, #0f1a24, #1a3040);
  color: #f0f4f8;
}

.xhs-cover-title {
  font-size: 26px;
  font-weight: 800;
  line-height: 1.3;
  margin-bottom: 12px;
}

.xhs-cover-badge {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 4px;
  background: rgba(15, 138, 161, 0.2);
  color: #8ddbd1;
  font-size: 13px;
  font-weight: 700;
}

.xhs-body {
  padding: 24px 28px;
  display: grid;
  gap: 16px;
}

.xhs-section-label {
  font-size: 11px;
  font-weight: 700;
  color: #8a7a6a;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: 4px;
}

.xhs-section-value {
  font-size: 15px;
  line-height: 1.5;
  color: #2a2018;
}

.xhs-risk .xhs-section-value {
  color: #b06040;
}

.xhs-footer {
  padding: 16px 28px;
  border-top: 1px solid #e5ddd0;
  font-size: 12px;
  color: #8a7a6a;
}

.share-actions {
  display: flex;
  gap: 8px;
  padding: 16px 20px;
  border-top: 1px solid var(--color-border-light);
}

.share-actions button {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
}
</style>
