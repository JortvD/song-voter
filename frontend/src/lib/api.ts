import type {
  AdminClearVotesResult,
  AdminDeleteUserResult,
  AdminForceLogoutResult,
  AdminInsights,
  AdminRenameUserResult,
  AdminResetPasswordResult,
  AdminSetDisabledResult,
  AdminSettings,
  AdminUsersPage,
  AggregateResponse,
  ChartFilters,
  ChartStatus,
  ChartStories,
  DashboardStats,
  DuelHistoryPage,
  FullResetResult,
  DuelHistoryStatus,
  ImportSummary,
  Invite,
  Matchup,
  MetadataStatus,
  PersonalRanking,
  PersonalSummary,
  Playlist,
  PlaylistOverview,
  PublicChart,
  SpotifyStatus,
  TrackDetail,
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
  signal?: AbortSignal
  timeoutMs?: number
}

const DEFAULT_TIMEOUT_MS = 15_000
// Keep this above the backend's 180s request limit so the server's timeout
// response wins and the UI can show the real error.
const LONG_RUNNING_TIMEOUT_MS = 200_000

function isErrorPayload(value: unknown): value is { error: string } {
  return (
    typeof value === 'object' &&
    value !== null &&
    'error' in value &&
    typeof (value as { error: unknown }).error === 'string'
  )
}

function withTimezone(path: string, timezone?: string) {
  if (!timezone) return path
  return `${path}${path.includes('?') ? '&' : '?'}tz=${encodeURIComponent(timezone)}`
}

async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { method = 'GET', body, signal, timeoutMs = DEFAULT_TIMEOUT_MS } = options
  const controller = new AbortController()
  let timedOut = false

  const forwardAbort = () => controller.abort()
  if (signal) {
    if (signal.aborted) controller.abort()
    else signal.addEventListener('abort', forwardAbort, { once: true })
  }

  const timer = window.setTimeout(() => {
    timedOut = true
    controller.abort()
  }, timeoutMs)

  try {
    const response = await fetch(path, {
      method,
      credentials: 'same-origin',
      headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: controller.signal,
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
  } catch (caught) {
    if (
      (caught instanceof DOMException || caught instanceof Error) &&
      caught.name === 'AbortError'
    ) {
      if (timedOut) {
        throw new ApiError('the request timed out; please try again', 0)
      }
      throw new ApiError('the request was cancelled', 0)
    }
    throw caught
  } finally {
    window.clearTimeout(timer)
    signal?.removeEventListener('abort', forwardAbort)
  }
}

function chartFilterQuery(filters: ChartFilters = {}): string {
  const params = new URLSearchParams({
    limit: String(filters.limit ?? 50),
    offset: String(filters.offset ?? 0),
    q: filters.q ?? '',
  })
  if (filters.decade != null) params.set('decade', String(filters.decade))
  if (filters.genre) params.set('genre', filters.genre)
  if (filters.artist) params.set('artist', filters.artist)
  return params.toString()
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
      timeoutMs: LONG_RUNNING_TIMEOUT_MS,
    }),
  schedulePlaylist: (playlist: string, startAt: string) =>
    request<Playlist>('/api/admin/playlist/schedule', {
      method: 'POST',
      body: { playlist, start_at: startAt },
    }),
  cancelScheduledPlaylist: () =>
    request<{ cancelled: boolean }>('/api/admin/playlist/schedule/cancel', {
      method: 'POST',
    }),
  startPlaylistNow: () =>
    request<ImportSummary>('/api/admin/playlist/start', {
      method: 'POST',
      timeoutMs: LONG_RUNNING_TIMEOUT_MS,
    }),
  setVotingPaused: (paused: boolean) =>
    request<{ voting_paused: boolean }>('/api/admin/voting/pause', {
      method: 'POST',
      body: { paused },
    }),
  backfillPreviews: () =>
    request<{ started: boolean; missing: number }>('/api/admin/previews/backfill', {
      method: 'POST',
    }),
  getTracks: (limit = 6, offset = 0) =>
    request<TracksPage>(`/api/admin/playlist/tracks?limit=${limit}&offset=${offset}`),
  getSpotifyStatus: () => request<SpotifyStatus>('/api/admin/spotify/status'),
  getSpotifyAuthorizeUrl: () =>
    request<{ url: string }>('/api/admin/spotify/authorize'),
  disconnectSpotify: () =>
    request<{ connected: boolean }>('/api/admin/spotify/disconnect', {
      method: 'POST',
    }),
  getMatchup: (timezone?: string, signal?: AbortSignal) =>
    request<Matchup>(withTimezone('/api/matchup', timezone), { signal }),
  vote: (input: { track_a: string; track_b: string; winner: string }) =>
    request<VoteResponse>('/api/vote', { method: 'POST', body: input }),
  undoVote: (timezone?: string) =>
    request<UndoResponse>(withTimezone('/api/vote/undo', timezone), {
      method: 'POST',
    }),
  getPersonalRanking: (
    offset = 0,
    limit = 50,
    filters: ChartFilters = {},
    signal?: AbortSignal,
  ) =>
    request<PersonalRanking>(
      `/api/me/ranking?${chartFilterQuery({ limit, offset, ...filters })}`,
      { signal },
    ),
  getPersonalSummary: (signal?: AbortSignal, timezone?: string) =>
    request<PersonalSummary>(withTimezone('/api/me/summary', timezone), { signal }),
  getDuelHistory: (
    offset = 0,
    limit = 50,
    status: DuelHistoryStatus = 'all',
    signal?: AbortSignal,
  ) =>
    request<DuelHistoryPage>(
      `/api/me/duels?limit=${limit}&offset=${offset}&status=${status}`,
      { signal },
    ),
  getChartStatus: (signal?: AbortSignal) =>
    request<ChartStatus>('/api/chart/status', { signal }),
  getPublicChart: (filters: ChartFilters = {}, signal?: AbortSignal) =>
    request<PublicChart>(`/api/chart?${chartFilterQuery(filters)}`, { signal }),
  getChartArtists: (filters: ChartFilters = {}, signal?: AbortSignal) =>
    request<AggregateResponse>(`/api/chart/artists?${chartFilterQuery(filters)}`, {
      signal,
    }),
  getChartAlbums: (filters: ChartFilters = {}, signal?: AbortSignal) =>
    request<AggregateResponse>(`/api/chart/albums?${chartFilterQuery(filters)}`, {
      signal,
    }),
  getChartStories: (signal?: AbortSignal) =>
    request<ChartStories>('/api/chart/stories', { signal }),
  getTrackDetail: (id: string, signal?: AbortSignal) =>
    request<TrackDetail>(`/api/tracks/${id}`, { signal }),
  getMetadataStatus: () => request<MetadataStatus>('/api/admin/metadata'),
  backfillMetadata: () =>
    request<{ run_id: string; started: boolean }>('/api/admin/metadata/backfill', {
      method: 'POST',
    }),
  setReveal: (revealed: boolean) =>
    request<ChartStatus>('/api/admin/reveal', {
      method: 'POST',
      body: { revealed },
    }),
  scheduleReveal: (revealAt: string, pauseVoting: boolean) =>
    request<ChartStatus>('/api/admin/reveal/schedule', {
      method: 'POST',
      body: { reveal_at: revealAt, pause_voting: pauseVoting },
    }),
  cancelScheduledReveal: () =>
    request<ChartStatus>('/api/admin/reveal/schedule/cancel', {
      method: 'POST',
    }),
  getDashboardStats: () => request<DashboardStats>('/api/admin/stats'),
  getAdminUsers: (offset = 0, limit = 50, query = '', sort = 'last_active') =>
    request<AdminUsersPage>(
      `/api/admin/users?limit=${limit}&offset=${offset}&q=${encodeURIComponent(query)}&sort=${encodeURIComponent(sort)}`,
    ),
  deleteAdminUser: (id: string, confirm: string) =>
    request<AdminDeleteUserResult>(`/api/admin/users/${id}`, {
      method: 'DELETE',
      body: { confirm },
    }),
  clearAdminUserVotes: (id: string, confirm: string) =>
    request<AdminClearVotesResult>(`/api/admin/users/${id}/clear-votes`, {
      method: 'POST',
      body: { confirm },
    }),
  renameAdminUser: (id: string, username: string) =>
    request<AdminRenameUserResult>(`/api/admin/users/${id}/username`, {
      method: 'PATCH',
      body: { username },
    }),
  resetAdminUserPassword: (id: string, password: string) =>
    request<AdminResetPasswordResult>(`/api/admin/users/${id}/password`, {
      method: 'POST',
      body: { password },
    }),
  setAdminUserDisabled: (id: string, disabled: boolean) =>
    request<AdminSetDisabledResult>(`/api/admin/users/${id}/disabled`, {
      method: 'POST',
      body: { disabled },
    }),
  logoutAdminUser: (id: string) =>
    request<AdminForceLogoutResult>(`/api/admin/users/${id}/logout`, {
      method: 'POST',
    }),
  getAdminInsights: (days = 14) =>
    request<AdminInsights>(`/api/admin/insights?days=${days}`),
  getAdminSettings: () => request<AdminSettings>('/api/admin/settings'),
  updateAdminSettings: (dailyStreakGoal: number) =>
    request<AdminSettings>('/api/admin/settings', {
      method: 'PATCH',
      body: { daily_streak_goal: dailyStreakGoal },
    }),
  getAdminStories: () => request<ChartStories>('/api/admin/stories'),
  getAdminRanking: (offset = 0, limit = 50, query = '') =>
    request<PublicChart>(
      `/api/admin/ranking?limit=${limit}&offset=${offset}&q=${encodeURIComponent(query)}`,
    ),
  revokeInvite: (id: string) =>
    request<{ revoked: boolean; id: string }>(`/api/admin/invites/${id}`, {
      method: 'DELETE',
    }),
  resetVotes: (confirm: string) =>
    request<{ reset: boolean; deleted_votes: number; deleted_matchups: number }>(
      '/api/admin/reset/votes',
      { method: 'POST', body: { confirm } },
    ),
  fullReset: (confirm: string) =>
    request<FullResetResult>('/api/admin/reset/all', {
      method: 'POST',
      body: { confirm },
    }),
  removePlaylist: (confirm: string) =>
    request<{ removed: boolean; removed_tracks: number; removed_votes: number }>(
      '/api/admin/playlist/remove',
      { method: 'POST', body: { confirm } },
    ),
}
