export type User = {
  id: string
  username: string
  is_admin: boolean
  created_at: string
}

export type Invite = {
  id: string
  code: string
  created_at: string
  used_at: string | null
  expires_at: string | null
  used_by_username: string | null
}

export type Playlist = {
  id: string
  spotify_id: string
  name: string
  description: string | null
  image_url: string | null
  owner_name: string | null
  imported_by: string | null
  imported_at: string
  reveal_state: 'hidden' | 'revealed'
  is_active: boolean
}

export type Track = {
  id: string
  playlist_id: string
  spotify_id: string
  title: string
  artists: string
  album: string | null
  album_image_url: string | null
  preview_url: string | null
  duration_ms: number | null
  position: number
  rating: number
  comparisons: number
  wins: number
}

export type PlaylistOverview = {
  playlist: Playlist | null
  track_count: number
}

export type ImportSummary = {
  playlist: Playlist
  imported_tracks: number
  skipped_tracks: number
  total_tracks: number
  previews_missing: number
}

export type TracksPage = {
  total: number
  limit: number
  offset: number
  tracks: Track[]
}

export type SpotifyStatus = {
  configured: boolean
  connected: boolean
  display_name: string | null
  connected_at: string | null
  expires_at: string | null
}

export type MatchupTrack = {
  id: string
  spotify_id: string
  title: string
  artists: string
  album: string | null
  album_image_url: string | null
  preview_url: string | null
  duration_ms: number | null
}

export type Matchup = {
  playlist_id: string
  playlist_name: string
  track_a: MatchupTrack
  track_b: MatchupTrack
  compared_pairs: number
  tracks_seen: number
  total_tracks: number
  votes_cast: number
}

export type VoteResponse = {
  vote_id: string
  playlist_id: string
  track_a: string
  track_b: string
  winner: string
  created_at: string
  votes_cast: number
}

export type UndoResponse = {
  undone_vote_id: string
  playlist_id: string
  track_a: MatchupTrack
  track_b: MatchupTrack
  winner: string
  created_at: string
  compared_pairs: number
  tracks_seen: number
  votes_cast: number
}

export type PersonalRankedTrack = {
  rank: number
  track: MatchupTrack
  rating: number
  comparisons: number
  wins: number
}

export type PersonalRanking = {
  playlist_id: string
  playlist_name: string
  total: number
  votes: number
  limit: number
  offset: number
  tracks: PersonalRankedTrack[]
}

export type ChartStatus = {
  revealed: boolean
  playlist_id: string | null
  playlist_name: string | null
  total_tracks: number
  votes_cast: number
}

export type ChartEntry = {
  rank: number
  track: MatchupTrack
  rating: number
  comparisons: number
  wins: number
}

export type PublicChart = {
  playlist_id: string
  playlist_name: string
  revealed: boolean
  total: number
  limit: number
  offset: number
  query: string
  tracks: ChartEntry[]
}

export type BalancedSettings = {
  influence_exponent: number
  confidence_prior: number
  neutral_prior: number
  min_picks: number
  updated_at: string
}

export type BalancedEntry = {
  rank: number | null
  track: MatchupTrack
  rating: number | null
  comparisons: number
  wins: number
  voters: number
  weight: number
  no_data: boolean
}

export type BalancedChart = {
  playlist_id: string
  playlist_name: string
  revealed: boolean
  total: number
  limit: number
  offset: number
  query: string
  tracks: BalancedEntry[]
}

export type DailyVotes = {
  date: string
  votes: number
}

export type DashboardStats = {
  playlist_id: string | null
  playlist_name: string | null
  revealed: boolean
  total_tracks: number
  tracks_compared: number
  total_users: number
  active_users: number
  total_votes: number
  votes_today: number
  active_today: number
  total_matchups: number
  daily: DailyVotes[]
}

export type AdminUserRow = {
  id: string
  username: string
  is_admin: boolean
  created_at: string
  votes_cast: number
  matchups_seen: number
  last_vote_at: string | null
  last_seen_at: string | null
  last_active_at: string
}

export type AdminUsersPage = {
  total: number
  limit: number
  offset: number
  query: string
  users: AdminUserRow[]
}
