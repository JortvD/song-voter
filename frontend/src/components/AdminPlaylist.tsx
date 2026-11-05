import { useCallback, useEffect, useState } from 'react'
import type { FormEvent } from 'react'

import { ApiError, api } from '../lib/api'
import type {
  ImportSummary,
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

function formatNumber(value: number): string {
  return value.toLocaleString()
}

export function AdminPlaylist() {
  const [overview, setOverview] = useState<PlaylistOverview | null>(null)
  const [spotify, setSpotify] = useState<SpotifyStatus | null>(null)
  const [tracks, setTracks] = useState<Track[]>([])
  const [loading, setLoading] = useState(true)
  const [input, setInput] = useState('')
  const [importing, setImporting] = useState(false)
  const [backfilling, setBackfilling] = useState(false)
  const [previewNotice, setPreviewNotice] = useState<string | null>(null)
  const [connecting, setConnecting] = useState(false)
  const [disconnecting, setDisconnecting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [noticeTone, setNoticeTone] = useState<'ok' | 'error'>('ok')
  const [result, setResult] = useState<ImportSummary | null>(null)

  const load = useCallback(async () => {
    const [data, status] = await Promise.all([
      api.getPlaylist(),
      api.getSpotifyStatus(),
    ])
    setOverview(data)
    setSpotify(status)

    if (data.playlist) {
      const page = await api.getTracks(6)
      setTracks(page.tracks)
    } else {
      setTracks([])
    }
  }, [])

  useEffect(() => {
    let active = true

    load()
      .catch((caught: unknown) => {
        if (active) {
          setError(caught instanceof ApiError ? caught.message : 'Could not load the playlist')
        }
      })
      .finally(() => {
        if (active) setLoading(false)
      })

    return () => {
      active = false
    }
  }, [load])

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

  async function handleImport(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setImporting(true)
    setError(null)
    setResult(null)
    setPreviewNotice(null)

    try {
      const summary = await api.importPlaylist(input.trim())
      setResult(summary)
      setInput('')
      await load()
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Import failed')
    } finally {
      setImporting(false)
    }
  }

  async function fetchPreviews() {
    setBackfilling(true)
    setError(null)
    setPreviewNotice(null)

    try {
      const response = await api.backfillPreviews()
      setPreviewNotice(
        response.missing > 0
          ? `Fetching ${formatNumber(response.missing)} missing previews in the background.`
          : 'All tracks already have previews.',
      )
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not start the preview fetch')
    } finally {
      setBackfilling(false)
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

  const playlist = overview?.playlist ?? null

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
              <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
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
                  : 'border-accent bg-accent/5 text-accent'
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
                    {formatNumber(overview?.track_count ?? 0)} tracks
                    {playlist.owner_name ? ` / ${playlist.owner_name}` : ''}
                  </p>
                  <p className="mt-0.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
                    Imported {formatDate(playlist.imported_at)}
                  </p>
                </div>
              </div>

              {tracks.length > 0 && (
                <ol className="mt-4 border-t border-line">
                  {tracks.map((track, index) => (
                    <li
                      key={track.id}
                      className="flex items-baseline gap-3 border-b border-line py-2"
                    >
                      <span className="w-5 shrink-0 font-mono text-[10px] text-muted">
                        {String(index + 1).padStart(2, '0')}
                      </span>
                      <div className="min-w-0">
                        <p className="truncate text-sm leading-tight">{track.title}</p>
                        <p className="truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                          {track.artists}
                        </p>
                      </div>
                    </li>
                  ))}
                </ol>
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
            {formatNumber(result.total_tracks)} tracks in the chart
            {result.skipped_tracks > 0
              ? ` — ${formatNumber(result.skipped_tracks)} unavailable items skipped`
              : ''}
            {result.previews_missing > 0
              ? ` — fetching ${formatNumber(result.previews_missing)} previews in the background`
              : ''}
            .
          </p>
        )}

        {previewNotice && (
          <p className="mt-4 border-l-2 border-ok bg-ok/5 px-3 py-2 text-xs text-ok">
            {previewNotice}
          </p>
        )}

        {error && (
          <p className="mt-4 border-l-2 border-accent bg-accent/5 px-3 py-2 text-xs text-accent">
            {error}
          </p>
        )}

        <form onSubmit={handleImport} className="mt-5 border-t border-line pt-4">
          <label htmlFor="playlist-url" className="label mb-1.5 block">
            Import from Spotify
          </label>
          <div className="flex flex-col gap-2 sm:flex-row">
            <input
              id="playlist-url"
              type="text"
              value={input}
              onChange={(event) => setInput(event.target.value)}
              placeholder="https://open.spotify.com/playlist/…"
              className="field flex-1 font-mono text-xs"
              required
            />
            <button
              type="submit"
              disabled={importing || input.trim().length === 0}
              className="btn shrink-0"
            >
              {importing ? 'Importing…' : 'Import'}
            </button>
          </div>
          <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
            Connect the playlist owner&apos;s account · re-import keeps ratings
          </p>
          <div className="mt-2 flex flex-wrap items-center justify-between gap-2">
            <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
              Previews are fetched from Spotify when missing
            </span>
            <button
              type="button"
              onClick={() => void fetchPreviews()}
              disabled={backfilling}
              className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted underline decoration-line underline-offset-2 transition hover:text-ink disabled:opacity-50"
            >
              {backfilling ? 'Fetching…' : 'Fetch missing previews'}
            </button>
          </div>
        </form>
      </div>
    </section>
  )
}
