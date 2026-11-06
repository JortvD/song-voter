import { useCallback, useEffect, useState } from 'react'
import type { FormEvent } from 'react'

import { usePlaylist } from '../context/PlaylistContext'
import { ApiError, api } from '../lib/api'
import type {
  ImportSummary,
  MetadataStatus,
  PlaylistOverview,
  SpotifyStatus,
  Track,
} from '../lib/types'

function formatDate(value: string): string {
  return new Date(value).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}

function formatDateTime(value: string | null): string {
  if (!value) return '—'
  return new Date(value).toLocaleString(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  })
}

function localInputValue(value: string | null): string {
  if (!value) return ''
  const date = new Date(value)
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000)
  return local.toISOString().slice(0, 16)
}

function formatNumber(value: number): string {
  return value.toLocaleString()
}

export function AdminPlaylist() {
  const { refresh: refreshPlaylist } = usePlaylist()
  const [overview, setOverview] = useState<PlaylistOverview | null>(null)
  const [spotify, setSpotify] = useState<SpotifyStatus | null>(null)
  const [tracks, setTracks] = useState<Track[]>([])
  const [metadata, setMetadata] = useState<MetadataStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [tracksTotal, setTracksTotal] = useState(0)
  const [tracksLoading, setTracksLoading] = useState(false)
  const [input, setInput] = useState('')
  const [startInput, setStartInput] = useState('')
  const [importing, setImporting] = useState(false)
  const [scheduling, setScheduling] = useState(false)
  const [starting, setStarting] = useState(false)
  const [scheduleNotice, setScheduleNotice] = useState<string | null>(null)
  const [backfilling, setBackfilling] = useState(false)
  const [metadataStarting, setMetadataStarting] = useState(false)
  const [previewNotice, setPreviewNotice] = useState<string | null>(null)
  const [previewPending, setPreviewPending] = useState(false)
  const [metadataNotice, setMetadataNotice] = useState<string | null>(null)
  const [connecting, setConnecting] = useState(false)
  const [disconnecting, setDisconnecting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [noticeTone, setNoticeTone] = useState<'ok' | 'error'>('ok')
  const [result, setResult] = useState<ImportSummary | null>(null)
  const [resultMode, setResultMode] = useState<'import' | 'resync'>('import')
  const [syncing, setSyncing] = useState(false)

  const playlist = overview?.playlist ?? null

  const load = useCallback(async () => {
    const [data, status, metadataStatus] = await Promise.all([
      api.getPlaylist(),
      api.getSpotifyStatus(),
      api.getMetadataStatus(),
    ])
    setOverview(data)
    setSpotify(status)
    setMetadata(metadataStatus)

    if (data.playlist) {
      setInput(data.playlist.spotify_id)
      setStartInput(localInputValue(data.playlist.scheduled_start_at))
      if (data.playlist.started_at) {
        const page = await api.getTracks(50)
        setTracks(page.tracks)
        setTracksTotal(page.total)
      } else {
        setTracks([])
        setTracksTotal(0)
      }
    } else {
      setTracks([])
      setTracksTotal(0)
      setInput('')
      setStartInput('')
    }
  }, [])

  useEffect(() => {
    let active = true
    setLoading(true)
    setLoadError(null)

    load()
      .catch((caught: unknown) => {
        if (active) {
          setLoadError(
            caught instanceof ApiError ? caught.message : 'Could not load the playlist',
          )
        }
      })
      .finally(() => {
        if (active) setLoading(false)
      })

    return () => {
      active = false
    }
  }, [load])

  function retryLoad() {
    setLoading(true)
    setLoadError(null)
    load()
      .catch((caught: unknown) => {
        setLoadError(
          caught instanceof ApiError ? caught.message : 'Could not load the playlist',
        )
      })
      .finally(() => setLoading(false))
  }

  useEffect(() => {
    const playlist = overview?.playlist
    if (!playlist || playlist.started_at) return

    const timer = window.setInterval(() => {
      api
        .getPlaylist()
        .then(async (next) => {
          if (next.playlist?.started_at) {
            await load()
          } else {
            setOverview(next)
          }
        })
        .catch(() => {
          // Keep the last known schedule; the manual refresh still works.
        })
    }, 15_000)

    return () => window.clearInterval(timer)
  }, [overview?.playlist?.started_at, load])

  useEffect(() => {
    if (metadata?.latest_run?.status !== 'running') return

    const timer = window.setInterval(() => {
      api
        .getMetadataStatus()
        .then((next) => setMetadata(next))
        .catch(() => {
          // Keep the last known progress; the manual refresh still works.
        })
    }, 2500)

    return () => window.clearInterval(timer)
  }, [metadata?.latest_run?.status])

  useEffect(() => {
    const params = new URLSearchParams(window.location.search)
    const outcome = params.get('spotify')
    if (!outcome) return

    if (outcome === 'connected') {
      setNotice('Spotify connected. You can import the playlist now.')
      setNoticeTone('ok')
    } else {
      setNotice('Spotify connection failed. Please try again.')
      setNoticeTone('error')
    }

    params.delete('spotify')
    const query = params.toString()
    window.history.replaceState(
      {},
      '',
      `${window.location.pathname}${query ? `?${query}` : ''}`,
    )
  }, [])

  async function startNow() {
    if (!input.trim() || importing) return
    setImporting(true)
    setError(null)
    setResult(null)
    setPreviewNotice(null)
    setScheduleNotice(null)

    try {
      const summary = await api.importPlaylist(input.trim())
      setResultMode('import')
      setResult(summary)
      await refreshPlaylist()
      await load()
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Import failed')
    } finally {
      setImporting(false)
    }
  }

  async function resyncPlaylist() {
    if (!playlist || syncing) return

    const confirmed = window.confirm(
      'Re-sync this Spotify playlist? Tracks and metadata will be refreshed; existing picks, duels and scores are preserved.',
    )
    if (!confirmed) return

    setSyncing(true)
    setError(null)
    setResult(null)
    setPreviewNotice(null)
    setScheduleNotice(null)

    try {
      const summary = await api.importPlaylist(playlist.spotify_id)
      setResultMode('resync')
      setResult(summary)
      await refreshPlaylist()
      await load()
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Re-sync failed')
    } finally {
      setSyncing(false)
    }
  }

  async function schedulePlaylist(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!input.trim() || !startInput || scheduling) return

    const startAt = new Date(startInput)
    if (Number.isNaN(startAt.getTime())) {
      setError('Choose a valid start date and time')
      return
    }

    setScheduling(true)
    setError(null)
    setScheduleNotice(null)
    setResult(null)

    try {
      const playlist = await api.schedulePlaylist(input.trim(), startAt.toISOString())
      setScheduleNotice(
        `Scheduled for ${formatDateTime(
          playlist?.scheduled_start_at ?? startAt.toISOString(),
        )}. Songs will be fetched when voting starts.`,
      )
      await refreshPlaylist()
      await load()
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : 'Could not schedule the playlist',
      )
    } finally {
      setScheduling(false)
    }
  }

  async function startScheduled() {
    if (starting) return
    setStarting(true)
    setError(null)
    setResult(null)
    setScheduleNotice(null)

    try {
      const summary = await api.startPlaylistNow()
      setResultMode('import')
      setResult(summary)
      await refreshPlaylist()
      await load()
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : 'Could not start the playlist',
      )
    } finally {
      setStarting(false)
    }
  }

  async function cancelSchedule() {
    if (starting) return
    setStarting(true)
    setError(null)
    setResult(null)
    setScheduleNotice(null)

    try {
      await api.cancelScheduledPlaylist()
      setScheduleNotice('Scheduled playlist cancelled.')
      await refreshPlaylist()
      await load()
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : 'Could not cancel the scheduled playlist',
      )
    } finally {
      setStarting(false)
    }
  }

  async function fetchPreviews() {
    setBackfilling(true)
    setPreviewPending(false)
    setError(null)
    setPreviewNotice(null)

    try {
      const response = await api.backfillPreviews()
      if (response.missing === 0) {
        setPreviewNotice('All tracks already have previews.')
        return
      }

      const initial = response.missing
      setPreviewNotice(
        `Fetching ${formatNumber(initial)} missing previews in the background…`,
      )
      setPreviewPending(true)

      // Preview backfill is a background task with no run id. Poll the
      // playlist overview until the gap count changes, then report the result
      // instead of leaving an optimistic "Fetching…" notice forever.
      let latest = initial
      for (let attempt = 0; attempt < 12; attempt += 1) {
        await new Promise((resolve) => window.setTimeout(resolve, 1800))
        const next = await api.getPlaylist()
        setOverview(next)
        latest = next.previews_missing
        if (latest === 0) {
          setPreviewNotice('Preview fetch complete — every track has a preview.')
          return
        }
        if (latest < initial) {
          setPreviewNotice(
            `Preview fetch finished — ${formatNumber(latest)} still missing.`,
          )
          return
        }
      }

      setPreviewNotice(
        `Preview fetch is still running in the background — ${formatNumber(latest)} still missing. Refresh to check again.`,
      )
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not start the preview fetch')
    } finally {
      setBackfilling(false)
      setPreviewPending(false)
    }
  }

  async function loadMoreTracks() {
    if (tracksLoading || tracks.length >= tracksTotal) return
    setTracksLoading(true)
    try {
      const page = await api.getTracks(50, tracks.length)
      setTracks((current) => [...current, ...page.tracks])
      setTracksTotal(page.total)
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not load more tracks')
    } finally {
      setTracksLoading(false)
    }
  }

  async function runMetadataBackfill() {
    setMetadataStarting(true)
    setError(null)
    setMetadataNotice(null)

    try {
      const response = await api.backfillMetadata()
      setMetadata(await api.getMetadataStatus())
      setMetadataNotice(
        response.started
          ? 'Backfilling release years, albums and genres in the background.'
          : 'A metadata backfill is already running.',
      )
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : 'Could not start the metadata backfill',
      )
    } finally {
      setMetadataStarting(false)
    }
  }

  async function connectSpotify() {
    setConnecting(true)
    setError(null)

    try {
      const { url } = await api.getSpotifyAuthorizeUrl()
      window.location.assign(url)
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not start Spotify connection')
      setConnecting(false)
    }
  }

  async function disconnectSpotify() {
    setDisconnecting(true)
    setError(null)

    try {
      await api.disconnectSpotify()
      setSpotify(await api.getSpotifyStatus())
      setNotice('Spotify disconnected.')
      setNoticeTone('ok')
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not disconnect Spotify')
    } finally {
      setDisconnecting(false)
    }
  }

  return (
    <section className="panel">
      <div className="flex items-center justify-between border-b border-line px-5 py-3">
        <h2 className="label">Playlist</h2>
        <span className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted">
          Spotify
        </span>
      </div>

      <div className="p-5">
        <div className="border border-line bg-paper p-3">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <p className="label">Spotify account</p>
              <p className="mt-1 truncate text-sm font-medium">
                {!spotify
                  ? 'Loading…'
                  : spotify.connected
                    ? (spotify.display_name ?? 'Connected')
                    : spotify.configured
                      ? 'Not connected'
                      : 'Not configured'}
              </p>
              <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
                {spotify?.connected
                  ? spotify.connected_at
                    ? `Since ${formatDate(spotify.connected_at)}`
                    : 'Connected'
                  : spotify?.configured
                    ? 'Required for Development Mode apps'
                    : 'Set SPOTIFY_REDIRECT_URI and restart'}
              </p>
            </div>

            {spotify?.connected ? (
              <button
                type="button"
                onClick={() => void disconnectSpotify()}
                disabled={disconnecting}
                className="btn-ghost shrink-0"
              >
                {disconnecting ? 'Working…' : 'Disconnect'}
              </button>
            ) : (
              <button
                type="button"
                onClick={() => void connectSpotify()}
                disabled={!spotify?.configured || connecting}
                className="btn shrink-0"
              >
                {connecting ? 'Opening…' : 'Connect'}
              </button>
            )}
          </div>

          {notice && (
            <p
              className={`mt-3 border-l-2 px-2 py-1 text-xs ${
                noticeTone === 'ok'
                  ? 'border-ok bg-ok/5 text-ok'
                  : 'border-danger bg-danger/5 text-danger'
              }`}
            >
              {notice}
            </p>
          )}
        </div>

        <div className="mt-5">
          {loading ? (
            <div className="space-y-2">
              <div className="h-16 animate-pulse border border-line bg-paper" />
              <div className="h-8 animate-pulse border border-line bg-paper" />
            </div>
          ) : loadError ? (
            <div role="alert" className="border border-danger/40 bg-danger/5 px-4 py-6 text-center">
              <p className="text-xs text-danger">{loadError}</p>
              <button type="button" onClick={retryLoad} className="btn-ghost mt-3">
                Try again
              </button>
            </div>
          ) : playlist ? (
            <>
              <div className="flex gap-4">
                {playlist.image_url ? (
                  <img
                    src={playlist.image_url}
                    alt=""
                    className="size-16 shrink-0 border border-line object-cover"
                  />
                ) : (
                  <div className="grid size-16 shrink-0 place-items-center border border-line bg-paper font-mono text-[9px] uppercase tracking-wider text-muted">
                    No art
                  </div>
                )}
                <div className="min-w-0">
                  <p className="truncate font-serif text-xl leading-tight">{playlist.name}</p>
                  <p className="mt-1 font-mono text-[10px] uppercase tracking-[0.14em] text-muted">
                    {playlist.started_at === null
                      ? 'Not loaded yet'
                      : `${formatNumber(overview?.track_count ?? 0)} tracks`}
                    {playlist.started_at !== null &&
                    (overview?.previews_missing ?? 0) > 0
                      ? ` · ${formatNumber(overview?.previews_missing ?? 0)} preview gaps`
                      : ''}
                    {playlist.owner_name ? ` / ${playlist.owner_name}` : ''}
                  </p>
                  <p className="mt-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
                    {playlist.started_at === null
                      ? `Set ${formatDate(playlist.imported_at)} · opens ${formatDateTime(playlist.scheduled_start_at)}`
                      : `Started ${formatDateTime(playlist.started_at)}`}
                  </p>
                  {playlist.voting_paused && (
                    <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-danger">
                      Voting paused
                    </p>
                  )}
                </div>
              </div>

              {playlist.started_at === null && (
                <div className="mt-4 border border-line bg-paper p-3">
                  <p className="label">Scheduled start</p>
                  <p className="mt-1 text-sm text-muted">
                    Voting opens at {formatDateTime(playlist.scheduled_start_at)}. The
                    song list is fetched then, so late additions are included.
                  </p>
                  {playlist.start_error && (
                    <p className="mt-2 break-words border-l-2 border-danger bg-danger/5 px-2 py-1 text-xs text-danger">
                      Last attempt: {playlist.start_error}
                    </p>
                  )}
                  <div className="mt-3 flex flex-wrap gap-2">
                    <button
                      type="button"
                      onClick={() => void startScheduled()}
                      disabled={starting || playlist.start_in_progress}
                      className="btn"
                    >
                      {playlist.start_in_progress
                        ? 'Starting…'
                        : starting
                          ? 'Starting…'
                          : 'Start now'}
                    </button>
                    <button
                      type="button"
                      onClick={() => void cancelSchedule()}
                      disabled={starting || playlist.start_in_progress}
                      className="btn-ghost"
                    >
                      Cancel schedule
                    </button>
                  </div>
                </div>
              )}

              {tracks.length > 0 && (
                <>
                  <ol className="mt-4 border-t border-line">
                    {tracks.map((track, index) => (
                      <li
                        key={track.id}
                        className="flex items-baseline gap-3 border-b border-line py-2"
                      >
                        <span className="w-5 shrink-0 font-mono text-[10px] text-muted">
                          {String(index + 1).padStart(2, '0')}
                        </span>
                        <div className="min-w-0 flex-1">
                          <p className="truncate text-sm leading-tight">{track.title}</p>
                          <p className="truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                            {track.artists}
                          </p>
                        </div>
                        {!track.preview_url && (
                          <span className="shrink-0 border border-danger/40 px-1.5 py-0.5 font-mono text-[8px] uppercase tracking-[0.1em] text-danger">
                            No preview
                          </span>
                        )}
                      </li>
                    ))}
                  </ol>
                  <div className="mt-3 flex flex-wrap items-center justify-between gap-2">
                    <span className="font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                      Showing {formatNumber(tracks.length)} of {formatNumber(tracksTotal)} tracks
                    </span>
                    {tracks.length < tracksTotal && (
                      <button
                        type="button"
                        onClick={() => void loadMoreTracks()}
                        disabled={tracksLoading}
                        className="btn-ghost"
                      >
                        {tracksLoading ? 'Loading…' : 'Load more tracks'}
                      </button>
                    )}
                  </div>
                </>
              )}
            </>
          ) : (
            <p className="border border-dashed border-line px-3 py-6 text-center text-xs text-muted">
              No playlist imported yet. Paste a Spotify playlist below.
            </p>
          )}
        </div>

        {result && (
          <p className="mt-4 border-l-2 border-ok bg-ok/5 px-3 py-2 text-xs text-ok">
            {formatNumber(result.total_tracks)} tracks{' '}
            {resultMode === 'resync' ? 'synced' : 'imported'}
            {result.skipped_tracks > 0
              ? ` — ${formatNumber(result.skipped_tracks)} unavailable items skipped`
              : ''}
            {result.previews_missing > 0
              ? ` — fetching ${formatNumber(result.previews_missing)} previews in the background`
              : ''}
            {result.metadata_backfill_started
              ? ' — metadata backfill queued'
              : ''}
            .
          </p>
        )}

        {previewNotice && (
          <p className="mt-4 border-l-2 border-ok bg-ok/5 px-3 py-2 text-xs text-ok">
            {previewNotice}
          </p>
        )}

        {metadata && metadata.tracks_total > 0 && (() => {
          const run = metadata.latest_run
          const running = run?.status === 'running'
          const percent =
            run && run.total_tracks > 0
              ? Math.min(
                  100,
                  Math.round(
                    (Math.min(run.processed_tracks, run.total_tracks) /
                      run.total_tracks) *
                      100,
                  ),
                )
              : 0

          return (
            <div className="mt-4 border border-line bg-paper p-3">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="min-w-0">
                  <p className="label">Metadata</p>
                  <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                    {formatNumber(metadata.tracks_with_year)}/
                    {formatNumber(metadata.tracks_total)} with release year ·{' '}
                    {formatNumber(metadata.tracks_with_genre)}/
                    {formatNumber(metadata.tracks_total)} with genre ·{' '}
                    {formatNumber(metadata.artists_total)} artists
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() => void runMetadataBackfill()}
                  disabled={metadataStarting || running}
                  className="btn-ghost shrink-0"
                >
                  {running
                    ? 'Backfilling…'
                    : metadataStarting
                      ? 'Starting…'
                      : 'Backfill metadata'}
                </button>
              </div>

              {run?.status === 'completed' && metadata.tracks_with_genre === 0 && (
                <p className="mt-2 border-l-2 border-line px-2 py-1 text-[11px] text-muted">
                  Spotify returned no artist genres. Release and album metadata are
                  still saved; genre facets stay hidden until genres are available.
                </p>
              )}

              {run && (
                <div className="mt-3">
                  <div className="flex flex-wrap items-center justify-between gap-2 font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                    <span
                      className={
                        run.status === 'failed'
                          ? 'text-danger'
                          : run.status === 'completed'
                            ? 'text-ok'
                            : 'text-ink'
                      }
                    >
                      {run.status} · {formatNumber(run.processed_tracks)}/
                      {formatNumber(run.total_tracks)} tracks ·{' '}
                      {formatNumber(run.processed_artists)}/
                      {formatNumber(run.total_artists)} artists
                    </span>
                    <span>{percent}%</span>
                  </div>
                  <div className="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-line">
                    <div
                      className="h-full rounded-full bg-ink transition-[width]"
                      style={{ width: `${percent}%` }}
                    />
                  </div>
                  {run.error && (
                    <p className="mt-2 break-words text-[11px] text-danger">{run.error}</p>
                  )}
                </div>
              )}

              {metadataNotice && (
                <p className="mt-3 border-l-2 border-ok bg-ok/5 px-2 py-1 text-xs text-ok">
                  {metadataNotice}
                </p>
              )}
            </div>
          )
        })()}

        {error && (
          <p className="mt-4 border-l-2 border-danger bg-danger/5 px-3 py-2 text-xs text-danger">
            {error}
          </p>
        )}

        {(!playlist || playlist.started_at === null) && (
          <form onSubmit={schedulePlaylist} className="mt-5 border-t border-line pt-4">
            <label htmlFor="playlist-url" className="label mb-1.5 block">
              {playlist ? 'Update the schedule' : 'Set a playlist'}
            </label>
            <div className="flex flex-col gap-2 sm:flex-row">
              <input
                id="playlist-url"
                type="text"
                value={input}
                onChange={(event) => setInput(event.target.value)}
                placeholder="https://open.spotify.com/playlist/…"
                className="field min-w-0 flex-1 font-mono text-xs"
                required
              />
              <input
                type="datetime-local"
                value={startInput}
                onChange={(event) => setStartInput(event.target.value)}
                className="field min-w-0 font-mono text-xs sm:w-56"
                aria-label="Scheduled start date and time"
                required
              />
            </div>
            <div className="mt-2 flex flex-wrap gap-2">
              <button
                type="submit"
                disabled={scheduling || input.trim().length === 0 || !startInput}
                className="btn shrink-0"
              >
                {scheduling
                  ? 'Scheduling…'
                  : playlist
                    ? 'Update schedule'
                    : 'Schedule'}
              </button>
              {!playlist && (
                <button
                  type="button"
                  onClick={() => void startNow()}
                  disabled={importing || input.trim().length === 0}
                  className="btn-ghost shrink-0"
                >
                  {importing ? 'Starting…' : 'Start now'}
                </button>
              )}
            </div>
            <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              Songs are fetched when voting starts, so late additions are included
              · connect the playlist owner&apos;s account
            </p>
          </form>
        )}

        {scheduleNotice && (
          <p className="mt-4 border-l-2 border-ok bg-ok/5 px-3 py-2 text-xs text-ok">
            {scheduleNotice}
          </p>
        )}

        {playlist?.started_at && (
          <div className="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-line pt-3">
            <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              Refresh the tape or fill preview gaps without touching picks
            </span>
            <div className="flex flex-wrap items-center gap-3">
              <button
                type="button"
                onClick={() => void resyncPlaylist()}
                disabled={syncing}
                className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted underline decoration-line underline-offset-2 transition hover:text-ink disabled:opacity-50"
              >
                {syncing ? 'Re-syncing…' : 'Re-sync playlist'}
              </button>
              <button
                type="button"
                onClick={() => void fetchPreviews()}
                disabled={backfilling || previewPending}
                className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted underline decoration-line underline-offset-2 transition hover:text-ink disabled:opacity-50"
              >
                {backfilling || previewPending ? 'Fetching…' : 'Fetch missing previews'}
              </button>
            </div>
          </div>
        )}
      </div>
    </section>
  )
}
