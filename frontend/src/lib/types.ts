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
  scheduled_start_at: string | null
  started_at: string | null
  voting_paused: boolean
  voting_paused_at: string | null
  scheduled_reveal_at: string | null
  pause_voting_on_reveal: boolean
  start_error: string | null
  start_attempts: number
  last_start_attempt_at: string | null
  start_in_progress: boolean
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
  release_date: string | null
  release_date_precision: string | null
  release_year: number | null
  decade: number | null
  album_id: string | null
  album_type: string | null
  total_tracks: number | null
  disc_number: number | null
  track_number: number | null
  primary_genre: string | null
  position: number
  rating: number
  comparisons: number
  wins: number
}

export type PlaylistOverview = {
  playlist: Playlist | null
  track_count: number
  previews_missing: number
}

export type ImportSummary = {
  playlist: Playlist
  imported_tracks: number
  skipped_tracks: number
  total_tracks: number
  previews_missing: number
  metadata_backfill_started: boolean
}

export type MetadataBackfillRun = {
  id: string
  status: 'running' | 'completed' | 'failed'
  total_tracks: number
  processed_tracks: number
  total_artists: number
  processed_artists: number
  started_at: string
  finished_at: string | null
  error: string | null
}

export type MetadataStatus = {
  playlist_id: string | null
  tracks_total: number
  tracks_with_year: number
  tracks_with_genre: number
  artists_total: number
  latest_run: MetadataBackfillRun | null
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
  release_year: number | null
  decade: number | null
  album_id: string | null
  album_type: string | null
  total_tracks: number | null
  disc_number: number | null
  track_number: number | null
  primary_genre: string | null
}

export type StreakDay = {
  date: string
  picks: number
  met: boolean
  is_today: boolean
}

export type StreakSummary = {
  current: number
  longest: number
  today_picks: number
  goal: number
  today_met: boolean
  at_risk: boolean
  days: StreakDay[]
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
  recent_duels: DuelHistoryItem[]
  streak: StreakSummary
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
  recent_duels: DuelHistoryItem[]
  streak: StreakSummary
}

export type DuelHistoryItem = {
  track_a: MatchupTrack
  track_b: MatchupTrack
  picked_track_id: string | null
  seen_at: string
  picked_at: string | null
}

export type DuelHistoryStatus = 'all' | 'picked' | 'skipped'

export type DuelHistoryPage = {
  playlist_id: string
  playlist_name: string
  total: number
  picked: number
  skipped: number
  filtered_total: number
  limit: number
  offset: number
  duels: DuelHistoryItem[]
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
  ranked_total: number
  top_score: number | null
  votes: number
  limit: number
  offset: number
  decades: FacetCount[]
  genres: FacetCount[]
  artists: ArtistFacet[]
  tracks: PersonalRankedTrack[]
}

export type PersonalSummaryTrack = {
  id: string
  title: string
  artists: string
  album_image_url: string | null
  rating: number
  comparisons: number
  wins: number
}

export type PersonalSummary = {
  playlist_id: string
  playlist_name: string
  votes: number
  total: number
  top_track: PersonalSummaryTrack | null
  streak: StreakSummary
}

export type FacetCount = {
  value: string
  count: number
  average_rating: number | null
}

export type ArtistFacet = {
  id: string
  name: string
  count: number
}

export type ChartStatus = {
  revealed: boolean
  playlist_id: string | null
  playlist_name: string | null
  playlist_description: string | null
  playlist_image_url: string | null
  total_tracks: number
  tracks_compared: number
  votes_cast: number
  voters: number
  duels: number
  total_artists: number
  total_albums: number
  year_min: number | null
  year_max: number | null
  scheduled_start_at: string | null
  started_at: string | null
  voting_paused: boolean
  voting_paused_at: string | null
  scheduled_reveal_at: string | null
  pause_voting_on_reveal: boolean
  decades: FacetCount[]
  genres: FacetCount[]
  artists: ArtistFacet[]
}

export type FullResetResult = {
  reset: boolean
  deleted_users: number
  deleted_invites: number
  deleted_playlists: number
  deleted_tracks: number
  deleted_votes: number
  deleted_duels: number
  deleted_artists: number
}

export type ChartFilters = {
  limit?: number
  offset?: number
  q?: string
  decade?: number | null
  genre?: string | null
  artist?: string | null
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
  decade: number | null
  genre: string | null
  tracks: ChartEntry[]
}

export type TrackHighlight = {
  id: string
  spotify_id: string
  title: string
  artists: string
  album_image_url: string | null
  rating: number | null
}

export type TrackDuel = {
  opponent: TrackHighlight
  won: boolean
  occurred_at: string
}

export type AlbumDetail = {
  id: string | null
  name: string
  artists: string
  image_url: string | null
  album_type: string | null
  release_year: number | null
  total_tracks: number | null
  playlist_track_count: number
  average_rating: number | null
  best_track: TrackHighlight | null
}

export type ArtistDetail = {
  id: string
  name: string
  genre: string | null
  track_count: number
  average_rating: number | null
}

export type OfficialTrackStats = {
  rank: number
  total_tracks: number
  rating: number
  total_rating_average: number | null
  comparisons: number
  wins: number
  losses: number
  win_rate: number | null
  voters: number
  unique_opponents: number
  first_duel_at: string | null
  last_duel_at: string | null
  recent_duels: TrackDuel[]
}

export type PersonalTrackStats = {
  rank: number | null
  total_ranked: number
  rating: number | null
  comparisons: number
  wins: number
  losses: number
  win_rate: number | null
  first_duel_at: string | null
  last_duel_at: string | null
  recent_duels: TrackDuel[]
}

export type TrackDetail = {
  playlist_id: string
  playlist_name: string
  revealed: boolean
  track: MatchupTrack
  album: AlbumDetail | null
  artists: ArtistDetail[]
  official: OfficialTrackStats | null
  personal: PersonalTrackStats | null
}

export type AggregateEntry = {
  id: string
  name: string
  subtitle: string | null
  image_url: string | null
  genre: string | null
  album_type: string | null
  release_year: number | null
  decade: number | null
  track_count: number
  rating: number
  comparisons: number
  wins: number
}

export type AggregateResponse = {
  playlist_id: string
  playlist_name: string
  revealed: boolean
  view: 'artists' | 'albums'
  total: number
  limit: number
  offset: number
  query: string
  decade: number | null
  genre: string | null
  entries: AggregateEntry[]
}

export type StoryStat = {
  label: string
  value: string
}

export type StoryTrack = {
  role: string
  id: string
  title: string
  artists: string
  album: string | null
  album_image_url: string | null
  rating: number
}

export type StoryCard = {
  id: string
  category: 'album' | 'artist' | 'decade' | 'duel' | 'tape'
  eyebrow: string
  title: string
  subtitle: string | null
  body: string
  image_url: string | null
  stats: StoryStat[]
  tracks: StoryTrack[]
}

export type StoryLeaderboardRow = {
  rank: number
  title: string
  subtitle: string | null
  image_url: string | null
  value: string
  detail: string
}

export type StoryLeaderboard = {
  id: string
  category: 'album' | 'artist' | 'decade' | 'duel'
  title: string
  metric_label: string
  detail_label: string
  rows: StoryLeaderboardRow[]
}

export type ChartStories = {
  playlist_id: string
  playlist_name: string
  revealed: boolean
  most_duelled: {
    track: MatchupTrack
    comparisons: number
  } | null
  top_voter: {
    username: string
    picks: number
  } | null
  cards: StoryCard[]
  leaderboards: StoryLeaderboard[]
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
  disabled_at: string | null
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
  sort: string
  users: AdminUserRow[]
}

export type AdminDeleteUserResult = {
  deleted: boolean
  id: string
  username: string
  deleted_votes: number
  deleted_duels: number
  rebuilt_playlists: number
  votes_replayed: number
  tracks_updated: number
}

export type AdminClearVotesResult = {
  cleared: boolean
  id: string
  username: string
  playlist_id: string
  playlist_name: string
  deleted_votes: number
  deleted_duels: number
  votes_replayed: number
  tracks_updated: number
}

export type AdminRenameUserResult = {
  updated: boolean
  id: string
  username: string
}

export type AdminResetPasswordResult = {
  updated: boolean
  id: string
  username: string
  signed_out: boolean
}

export type AdminSetDisabledResult = {
  updated: boolean
  id: string
  username: string
  disabled: boolean
  signed_out: boolean
}

export type AdminForceLogoutResult = {
  signed_out: boolean
  id: string
  username: string
}

export type AdminHourlyPicks = {
  hour: number
  picks: number
}

export type AdminPickBucket = {
  label: string
  voters: number
}

export type AdminTopTrack = {
  id: string
  title: string
  artists: string
  album_image_url: string | null
  rating: number
  comparisons: number
  wins: number
}

export type AdminTopVoter = {
  user_id: string
  username: string
  picks: number
  duels: number
  last_active_at: string
}

export type AdminRecentPick = {
  created_at: string
  username: string
  winner_title: string
  winner_artists: string
  winner_image_url: string | null
  other_title: string
  other_artists: string
}

export type AdminDailyVotes = {
  date: string
  votes: number
  voters: number
}

export type AdminSettings = {
  daily_streak_goal: number
  min: number
  max: number
}

export type AdminInsights = {
  playlist_id: string | null
  playlist_name: string | null
  playlist_image_url: string | null
  revealed: boolean
  range_days: number
  range_start: string
  range_end: string
  range_picks: number
  range_voters: number
  range_duels: number
  range_picks_per_voter: number
  range_duels_per_voter: number
  total_tracks: number
  tracks_compared: number
  tracks_uncompared: number
  previews_missing: number
  total_picks: number
  picks_today: number
  picks_7d: number
  picks_per_voter: number
  total_voters: number
  active_voters_24h: number
  active_voters_7d: number
  total_users: number
  admins: number
  invites_total: number
  invites_used: number
  invites_open: number
  invites_expired: number
  total_duels: number
  duels_per_voter: number
  coverage_pct: number
  daily: AdminDailyVotes[]
  hourly: AdminHourlyPicks[]
  pick_distribution: AdminPickBucket[]
  top_tracks: AdminTopTrack[]
  top_voters: AdminTopVoter[]
  recent_picks: AdminRecentPick[]
}
