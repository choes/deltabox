export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let value = bytes
  let unit = 'B'
  for (const u of units) {
    if (value < 1024) break
    value /= 1024
    unit = u
  }
  return `${value.toFixed(1)} ${unit}`
}

export function formatTime(iso: string | null | undefined): string {
  if (!iso) return '-'
  return iso.replace('T', ' ').replace(/([+-]\d{2}:\d{2}|Z)$/, '').slice(0, 19)
}

const SOURCE_LABELS: Record<string, string> = {
  plain_text: '文本',
  pdf_text: 'PDF',
  docx_text: 'DOCX',
  docx_header_footer: 'DOCX 页眉页脚',
  xlsx_text: 'XLSX',
  pptx_text: 'PPTX',
  pptx_speaker_notes: 'PPTX 备注',
  image_metadata: '图片元数据',
  video_metadata: '视频元数据',
}

export function sourceLabel(source: string | null | undefined): string {
  if (!source) return ''
  return SOURCE_LABELS[source] ?? source
}

export function matchKindLabel(kind: string): string {
  const labels: Record<string, string> = {
    name: '文件名',
    path: '路径',
    tag: '标签',
    text: '内容',
  }
  return labels[kind] ?? kind
}

export function jobStatusLabel(status: string): string {
  const labels: Record<string, string> = {
    pending: '等待中',
    running: '运行中',
    paused: '已暂停',
    completed: '已完成',
    failed_retryable: '失败(可重试)',
    failed_permanent: '失败',
    skipped: '已跳过',
    cancelled: '已取消',
  }
  return labels[status] ?? status
}
