// API types mirroring deltabox-core record types.

export interface FileRecord {
  file_id: string
  name: string
  logical_path: string
  size: number
  content_hash: string
  status: 'active' | 'trashed' | 'purged' | 'incomplete'
  imported_at: string
  trashed_at: string | null
}

export interface SearchMatch {
  match_kind: 'name' | 'path' | 'tag' | 'text'
  source: string | null
  text: string | null
  page: number | null
  line_start: number | null
  line_end: number | null
  score: number | null
}

export interface SearchResult {
  file: FileRecord
  matches: SearchMatch[]
}

export interface TagRecord {
  tag_id: string
  name: string
  tag_type: string
  source: string
  created_at: string
  updated_at: string
}

export interface IndexJobRecord {
  job_id: string
  file_id: string
  job_type: string
  status: string
  total_tasks: number
  completed_tasks: number
  failed_tasks: number
  created_at: string
  updated_at: string
  last_error: string | null
}

export interface TextSegmentRecord {
  segment_id: string
  file_id: string
  source: string
  task_key: string
  segment_index: number
  text: string
  page: number | null
  line_start: number | null
  line_end: number | null
  start_ms: number | null
  end_ms: number | null
  confidence: number
  created_at: string
  updated_at: string
}

export interface FileManifest {
  file_id: string
  name: string
  logical_path: string
  mime: string
  size: number
  content_hash: string
  version: number
  status: string
  created_at: string
  modified_at: string
  imported_at: string
  trashed_at: string | null
  tags: Array<{ name: string; tag_type: string; source: string }>
}

export interface IndexRunSummary {
  completed: number
  failed: number
  skipped: number
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`/api${path}`, init)
  if (!res.ok) {
    let message = `HTTP ${res.status}`
    try {
      const body = (await res.json()) as { error?: string }
      if (body.error) message = body.error
    } catch {
      // keep HTTP status message
    }
    throw new Error(message)
  }
  const text = await res.text()
  return (text ? JSON.parse(text) : null) as T
}

function jsonBody(data: unknown): RequestInit {
  return {
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  }
}

export const api = {
  // files
  listFiles: () => request<FileRecord[]>('/files'),
  fileInfo: (id: string) => request<FileManifest>(`/files/${encodeURIComponent(id)}`),
  uploadFile(file: File, path?: string): Promise<FileManifest> {
    const form = new FormData()
    if (path) form.append('path', path)
    form.append('file', file)
    return request<FileManifest>('/files', { method: 'POST', body: form })
  },
  downloadUrl: (id: string) => `/api/files/${encodeURIComponent(id)}/download`,
  trashFile: (id: string) =>
    request<FileManifest>(`/files/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  fileSegments: (id: string) =>
    request<TextSegmentRecord[]>(`/files/${encodeURIComponent(id)}/segments`),

  // search
  search: (q: string) => request<FileRecord[]>(`/search?q=${encodeURIComponent(q)}`),
  searchDetailed: (q: string) =>
    request<SearchResult[]>(`/search?q=${encodeURIComponent(q)}&details=true`),

  // tags
  listTags: () => request<TagRecord[]>('/tags'),
  createTag: (name: string, tagType?: string) =>
    request<TagRecord>('/tags', { method: 'POST', ...jsonBody({ name, tag_type: tagType }) }),
  renameTag: (name: string, newName: string) =>
    request<TagRecord>(`/tags/${encodeURIComponent(name)}`, {
      method: 'PUT',
      ...jsonBody({ new_name: newName }),
    }),
  deleteTag: (name: string) =>
    request<null>(`/tags/${encodeURIComponent(name)}`, { method: 'DELETE' }),
  fileTags: (fileId: string) =>
    request<TagRecord[]>(`/files/${encodeURIComponent(fileId)}/tags`),
  attachTag: (fileId: string, name: string) =>
    request<TagRecord>(`/files/${encodeURIComponent(fileId)}/tags`, {
      method: 'POST',
      ...jsonBody({ name }),
    }),
  detachTag: (fileId: string, name: string) =>
    request<null>(
      `/files/${encodeURIComponent(fileId)}/tags/${encodeURIComponent(name)}`,
      { method: 'DELETE' },
    ),

  // index jobs
  listIndexJobs: () => request<IndexJobRecord[]>('/index/jobs'),
  indexJobAction: (id: string, action: 'pause' | 'resume' | 'retry' | 'cancel') =>
    request<IndexJobRecord>(
      `/index/jobs/${encodeURIComponent(id)}/${action}`,
      { method: 'POST' },
    ),
  runIndexWorker: (limit = 10) =>
    request<IndexRunSummary>(`/index/run?limit=${limit}`, { method: 'POST' }),
}
