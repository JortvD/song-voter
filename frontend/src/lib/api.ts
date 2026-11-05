import type {
  AdminUsersPage,
  ChartStatus,
  DashboardStats,
  ImportSummary,
  Invite,
  Matchup,
  PersonalRanking,
  PlaylistOverview,
  PublicChart,
  SpotifyStatus,
  TracksPage,
  UndoResponse,
  User,
  VoteResponse,
} from './types'

export class ApiError extends Error {
  readonly status: number

  constructor(message: string, status: number) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

type RequestOptions = {
  method?: string
  body?: unknown
}

function isErrorPayload(value: unknown): value is { error: string } {
  return (
    typeof value === 'object' &&
    value !== null &&
    'error' in value &&
    typeof (value as { error: unknown }).error === 'string'
  )
}

async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { method = 'GET', body } = options

  const response = await fetch(path, {
    method,
    credentials: 'same-origin',
    headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })

  const text = await response.text()
  let payload: unknown = null

  if (text) {
    try {
      payload = JSON.parse(text)
    } catch {
      payload = null
    }
  }

  if (!response.ok) {
    if (response.status === 401 && !path.startsWith('/api/auth/')) {
      // The session expired or was revoked; let the auth provider reset itself.
      window.dispatchEvent(new Event('ull:unauthorized'))
    }

    const message = isErrorPayload(payload)
      ? payload.error
      : `request failed with status ${response.status}`
    throw new ApiError(message, response.status)
  }

  return payload as T
}

export const api = {
  me: () => request<User>('/api/auth/me'),
  login: (input: { username: string; password: string }) =>
    request<User>('/api/auth/login', { method: 'POST', body: input }),
  register: (input: { username: string; password: string; invite_code: string }) =>
    request<User>('/api/auth/register', { method: 'POST', body: input }),
  logout: () => request<void>('/api/auth/logout', { method: 'POST' }),
  listInvites: () => request<Invite[]>('/api/admin/invites'),
  createInvite: () => request<Invite>('/api/admin/invites', { method: 'POST' }),
  getPlaylist: () => request<PlaylistOverview>('/api/admin/playlist'),
  importPlaylist: (playlist: string) =>
    request<ImportSummary>('/api/admin/playlist/import', {
      method: 'POST',
      body: { playlist },
    }),
  backfillPreviews: () =>
    request<{ started: boolean; missing: number }>('/api/admin/previews/backfill', {
      method: 'POST',
    }),
  getTracks: (limit = 6) =>
    request<TracksPage>(`/api/admin/playlist/tracks?limit=${limit}`),
  getSpotifyStatus: () => request<SpotifyStatus>('/api/admin/spotify/status'),
  getSpotifyAuthorizeUrl: () =>
    request<{ url: string }>('/api/admin/spotify/authorize'),
  disconnectSpotify: () =>
    request<{ connected: boolean }>('/api/admin/spotify/disconnect', {
      method: 'POST',
    }),
  getMatchup: () => request<Matchup>('/api/matchup'),
  vote: (input: { track_a: string; track_b: string; winner: string }) =>
    request<VoteResponse>('/api/vote', { method: 'POST', body: input }),
  undoVote: () => request<UndoResponse>('/api/vote/undo', { method: 'POST' }),
  getPersonalRanking: (offset = 0, limit = 50) =>
    request<PersonalRanking>(`/api/me/ranking?limit=${limit}&offset=${offset}`),
  getChartStatus: () => request<ChartStatus>('/api/chart/status'),
  getPublicChart: (offset = 0, limit = 50, query = '') =>
    request<PublicChart>(
      `/api/chart?limit=${limit}&offset=${offset}&q=${encodeURIComponent(query)}`,
    ),
  setReveal: (revealed: boolean) =>
    request<ChartStatus>('/api/admin/reveal', {
      method: 'POST',
      body: { revealed },
    }),
  getDashboardStats: () => request<DashboardStats>('/api/admin/stats'),
  getAdminUsers: (offset = 0, limit = 50, query = '') =>
    request<AdminUsersPage>(
      `/api/admin/users?limit=${limit}&offset=${offset}&q=${encodeURIComponent(query)}`,
    ),
  getAdminRanking: (offset = 0, limit = 50, query = '') =>
    request<PublicChart>(
      `/api/admin/ranking?limit=${limit}&offset=${offset}&q=${encodeURIComponent(query)}`,
    ),
  resetVotes: (confirm: string) =>
    request<{ reset: boolean; deleted_votes: number; deleted_matchups: number }>(
      '/api/admin/reset/votes',
      { method: 'POST', body: { confirm } },
    ),
}
