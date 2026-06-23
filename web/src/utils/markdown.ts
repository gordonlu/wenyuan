export function renderMarkdown(text: string): string {
  let html = text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')

  html = html
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/__(.+?)__/g, '<em>$1</em>')
    .replace(/^### (.+)$/gm, '<h4>$1</h4>')
    .replace(/^## (.+)$/gm, '<h3>$1</h3>')
    .replace(/^# (.+)$/gm, '<h2>$1</h2>')

  const lines = html.split('\n')
  const result: string[] = []
  let inList = false

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]
    const trimmed = line.trim()

    if (!trimmed) {
      if (inList) { result.push('</ul>'); inList = false }
      continue
    }

    if (/^[-*]\s/.test(trimmed)) {
      if (!inList) { result.push('<ul>'); inList = true }
      result.push(`<li>${trimmed.replace(/^[-*]\s/, '')}</li>`)
      continue
    }

    if (/^\d+[.)]\s/.test(trimmed)) {
      if (!inList) { result.push('<ol>'); inList = true }
      result.push(`<li>${trimmed.replace(/^\d+[.)]\s/, '')}</li>`)
      continue
    }

    if (inList) { result.push('</ul>'); inList = false }

    if (/^<h\d>/.test(trimmed) || /^<\/?[ou]l>/.test(trimmed)) {
      result.push(trimmed)
    } else {
      result.push(`<p>${trimmed}</p>`)
    }
  }

  if (inList) result.push('</ul>')

  return result.join('\n')
}
