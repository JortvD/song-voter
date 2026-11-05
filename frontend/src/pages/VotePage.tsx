import { useCallback, useEffect, useRef, useState } from 'react'
import type { RefObject } from 'react'
import { Link } from 'react-router-dom'

import { Kbd, PreviewPlayer } from '../components/PreviewPlayer'
import { RequireAuth } from '../components/RequireAuth'
import { useAuth } from '../context/AuthContext'
import { ApiError, api } from '../lib/api'
import type { Matchup, MatchupTrack } from '../lib/types'

function sleep(milliseconds: number) {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds))
}

function formatNumber(value: number) {
  return value.toLocaleString()
}

type MatchupCardProps = {
  side: 'a' | 'b'
  track: MatchupTrack
  picked: boolean
  disabled: boolean
  audioRef: RefObject<HTMLAudioElement | null>
  onPlayStateChange: (playing: boolean) => void
  onPick: () => void
}

function MatchupCard({
  side,
  track,
  picked,
  disabled,
  audioRef,
  onPlayStateChange,
  onPick,
}: MatchupCardProps) {
  return (
    <article
      className={`panel flex flex-col transition duration-150 ${
        picked ? 'border-accent shadow-[3px_3px_0_0_#cf4a2f]' : ''
      }`}
    >
      <div className="flex items-center justify-between gap-2 border-b border-line px-3.5 py-2">
        <span className="label">{side === 'a' ? 'Track A' : 'Track B'}</span>
        <span className="flex items-center gap-1.5 font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
          <Kbd>{side === 'a' ? 'A' : 'B'}</Kbd> play
          <span className="text-muted/40">·</span>
          <Kbd>{side === 'a' ? '1' : '2'}</Kbd> pick
        </span>
      </div>

      <div className="flex flex-1 flex-col p-3 sm:p-3.5">
        <div className="flex gap-3">
          {track.album_image_url ? (
            <img
              src={track.album_image_url}
              alt=""
              className="size-24 shrink-0 border border-line object-cover sm:size-28"
            />
          ) : (
            <div className="grid size-24 shrink-0 place-items-center border border-line bg-paper font-serif text-3xl text-muted/40 sm:size-28">
              ♪
            </div>
          )}

          <div className="min-w-0 flex-1">
            <h2 className="line-clamp-2 font-serif text-xl leading-tight sm:text-2xl">
              {track.title}
            </h2>
            <p className="mt-1.5 line-clamp-2 font-mono text-[9px] uppercase leading-relaxed tracking-[0.12em] text-muted">
              {track.artists}
            </p>
            {track.album && (
              <p className="mt-1 truncate text-[11px] text-muted/70">{track.album}</p>
            )}
          </div>
        </div>

        <div className="mt-2.5">
          <PreviewPlayer
            previewUrl={track.preview_url}
            spotifyId={track.spotify_id}
            title={track.title}
            audioRef={audioRef}
            onPlayStateChange={onPlayStateChange}
          />
        </div>

        <button
          type="button"
          onClick={onPick}
          disabled={disabled}
          className={`btn mt-2.5 w-full ${picked ? 'border-accent bg-accent' : ''}`}
        >
          {picked ? 'Picked' : 'Pick this one'}
        </button>
      </div>
    </article>
  )
}

function LegendItem({ keys, label }: { keys: string[]; label: string }) {
  return (
    <span className="flex items-center gap-1 font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
      {keys.map((key, index) => (
        <span key={key} className="flex items-center gap-1">
          {index > 0 && <span className="text-muted/40">/</span>}
          <Kbd>{key}</Kbd>
        </span>
      ))}
      {label}
    </span>
  )
}

function ShortcutLegend() {
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5 border border-line bg-card px-3 py-1.5">
      <span className="label">Keys</span>
      <LegendItem keys={['A']} label="play A" />
      <LegendItem keys={['1', '←']} label="pick A" />
      <LegendItem keys={['B']} label="play B" />
      <LegendItem keys={['2', '→']} label="pick B" />
      <LegendItem keys={['Z']} label="undo" />
      <span className="basis-full font-mono text-[8px] uppercase tracking-[0.12em] text-muted/60">
        Shortcuts work while this page has focus.
      </span>
    </div>
  )
}

function VoteScreen() {
  const { user } = useAuth()
  const [matchup, setMatchup] = useState<Matchup | null>(null)
  const [loading, setLoading] = useState(true)
  const [voting, setVoting] = useState(false)
  const [undoing, setUndoing] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [exhausted, setExhausted] = useState(false)
  const [pickedId, setPickedId] = useState<string | null>(null)
  const [sessionPicks, setSessionPicks] = useState(0)
  const [votesCast, setVotesCast] = useState(0)
  const [totalTracks, setTotalTracks] = useState(0)
  const [tracksSeen, setTracksSeen] = useState(0)
  const [comparedPairs, setComparedPairs] = useState(0)
  const [playlistName, setPlaylistName] = useState('')

  const audioA = useRef<HTMLAudioElement | null>(null)
  const audioB = useRef<HTMLAudioElement | null>(null)

  const onPlayA = useCallback((playing: boolean) => {
    if (playing) audioB.current?.pause()
  }, [])
  const onPlayB = useCallback((playing: boolean) => {
    if (playing) audioA.current?.pause()
  }, [])

  const applyMatchup = useCallback((next: Matchup) => {
    setMatchup(next)
    setVotesCast(next.votes_cast)
    setTotalTracks(next.total_tracks)
    setTracksSeen(next.tracks_seen)
    setComparedPairs(next.compared_pairs)
    setPlaylistName(next.playlist_name)
    setExhausted(false)
    setError(null)
  }, [])

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)

    try {
      applyMatchup(await api.getMatchup())
    } catch (caught) {
      if (caught instanceof ApiError && caught.status === 409) {
        setExhausted(true)
        setMatchup(null)
      } else {
        setError(caught instanceof ApiError ? caught.message : 'Could not load a matchup')
        setMatchup(null)
      }
    } finally {
      setLoading(false)
    }
  }, [applyMatchup])

  useEffect(() => {
    if (user) void load()
  }, [user, load])

  const vote = useCallback(
    async (trackId: string) => {
      if (!matchup || voting || undoing) return

      setVoting(true)
      setError(null)
      setPickedId(trackId)

      try {
        const response = await api.vote({
          track_a: matchup.track_a.id,
          track_b: matchup.track_b.id,
          winner: trackId,
        })

        setVotesCast(response.votes_cast)
        setSessionPicks((value) => value + 1)

        await sleep(220)

        try {
          applyMatchup(await api.getMatchup())
        } catch (caught) {
          if (caught instanceof ApiError && caught.status === 409) {
            setExhausted(true)
            setMatchup(null)
          } else {
            setError(caught instanceof ApiError ? caught.message : 'Could not load the next matchup')
            setMatchup(null)
          }
        }
      } catch (caught) {
        setError(caught instanceof ApiError ? caught.message : 'Vote failed')
      } finally {
        setVoting(false)
        setPickedId(null)
      }
    },
    [matchup, voting, undoing, applyMatchup],
  )

  const undo = useCallback(async () => {
    if (undoing || voting || votesCast === 0) return

    setUndoing(true)
    setError(null)

    try {
      const undone = await api.undoVote()

      setVotesCast(undone.votes_cast)
      setSessionPicks((value) => Math.max(0, value - 1))
      setComparedPairs(undone.compared_pairs)
      setTracksSeen(undone.tracks_seen)
      setExhausted(false)
      setMatchup({
        playlist_id: undone.playlist_id,
        playlist_name: playlistName || 'The chart',
        track_a: undone.track_a,
        track_b: undone.track_b,
        compared_pairs: undone.compared_pairs,
        tracks_seen: undone.tracks_seen,
        total_tracks: totalTracks,
        votes_cast: undone.votes_cast,
      })
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Undo failed')
    } finally {
      setUndoing(false)
    }
  }, [undoing, voting, votesCast, playlistName, totalTracks])

  const playSide = useCallback(
    (side: 'a' | 'b') => {
      if (!matchup) return

      const previewUrl =
        side === 'a' ? matchup.track_a.preview_url : matchup.track_b.preview_url
      const target = side === 'a' ? audioA.current : audioB.current

      if (!previewUrl || !target) {
        setError('No Spotify preview is available for this song. Use “Open in Spotify” instead.')
        return
      }

      if (!target.paused) {
        target.pause()
        return
      }

      const other = side === 'a' ? audioB.current : audioA.current
      other?.pause()

      void target
        .play()
        .then(() => setError(null))
        .catch(() => setError('Could not play the Spotify preview.'))
    },
    [matchup],
  )

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (!matchup || voting || undoing || event.repeat) return
      if (event.metaKey || event.ctrlKey || event.altKey) return

      const target = event.target as HTMLElement | null
      if (
        target &&
        (target.tagName === 'INPUT' ||
          target.tagName === 'TEXTAREA' ||
          target.isContentEditable)
      ) {
        return
      }

      const key = event.key.toLowerCase()

      if (key === '1' || event.key === 'ArrowLeft') {
        event.preventDefault()
        void vote(matchup.track_a.id)
      } else if (key === '2' || event.key === 'ArrowRight') {
        event.preventDefault()
        void vote(matchup.track_b.id)
      } else if (key === 'a') {
        event.preventDefault()
        playSide('a')
      } else if (key === 'b') {
        event.preventDefault()
        playSide('b')
      } else if (key === 'z' && votesCast > 0) {
        event.preventDefault()
        void undo()
      }
    }

    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [matchup, voting, undoing, votesCast, vote, undo, playSide])

  const progress =
    totalTracks > 0 ? Math.min(100, Math.round((tracksSeen / totalTracks) * 100)) : 0

  return (
    <>
      <div className="flex flex-wrap items-end justify-between gap-3 border-b border-line pb-2.5">
        <div>
          <p className="label">Matchup</p>
          <h1 className="mt-1 font-serif text-2xl leading-tight sm:text-3xl">
            {playlistName || 'The chart'}
          </h1>
          <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {formatNumber(comparedPairs)} matchups · {formatNumber(votesCast)} picks ·{' '}
            {formatNumber(sessionPicks)} this session
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Link to="/ranking" className="btn-ghost">
            Your chart
          </Link>
          <button
            type="button"
            onClick={() => void undo()}
            disabled={undoing || voting || votesCast === 0}
            className="btn-ghost"
          >
            {undoing ? 'Undoing…' : 'Undo'}
            <Kbd>Z</Kbd>
          </button>
        </div>
      </div>

      {matchup && totalTracks > 0 && (
        <div className="mt-2.5 grid gap-3 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-center">
          <div>
            <div className="flex items-center justify-between font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              <span>
                {formatNumber(tracksSeen)} of {formatNumber(totalTracks)} songs seen
              </span>
              <span>{progress}%</span>
            </div>
            <div className="mt-1.5 h-1 w-full bg-line">
              <div
                className="h-1 bg-ink transition-[width] duration-300"
                style={{ width: `${progress}%` }}
              />
            </div>
          </div>
          <ShortcutLegend />
        </div>
      )}

      {error && (
        <p role="alert" className="mt-3 border-l-2 border-accent bg-accent/5 px-3 py-2 text-sm text-accent">
          {error}
        </p>
      )}

      <div className="mt-3">
        {loading ? (
          <div className="grid gap-3 lg:grid-cols-[1fr_32px_1fr]">
            <div className="panel h-80 animate-pulse" />
            <div className="hidden lg:block" />
            <div className="panel h-80 animate-pulse" />
          </div>
        ) : exhausted ? (
          <section className="panel p-6 text-center sm:p-8">
            <p className="label">Chart complete</p>
            <h2 className="mt-2 font-serif text-3xl">You&apos;ve seen every pair.</h2>
            <p className="mx-auto mt-2 max-w-md text-sm leading-relaxed text-muted">
              Your chart has {formatNumber(votesCast)} picks. The shared ranking
              stays sealed until the reveal.
            </p>
            <div className="mt-5 flex flex-wrap justify-center gap-3">
              <Link to="/ranking" className="btn">
                Open your chart
              </Link>
              <button
                type="button"
                onClick={() => void undo()}
                disabled={undoing || votesCast === 0}
                className="btn-ghost"
              >
                Undo last pick
              </button>
            </div>
          </section>
        ) : matchup ? (
          <div className="grid items-stretch gap-3 lg:grid-cols-[1fr_32px_1fr]">
            <MatchupCard
              side="a"
              track={matchup.track_a}
              picked={pickedId === matchup.track_a.id}
              disabled={voting || undoing}
              audioRef={audioA}
              onPlayStateChange={onPlayA}
              onPick={() => void vote(matchup.track_a.id)}
            />

            <div className="hidden items-center justify-center lg:flex">
              <span className="grid size-8 place-items-center border border-line bg-card font-mono text-[9px] uppercase tracking-wider text-muted">
                vs
              </span>
            </div>

            <MatchupCard
              side="b"
              track={matchup.track_b}
              picked={pickedId === matchup.track_b.id}
              disabled={voting || undoing}
              audioRef={audioB}
              onPlayStateChange={onPlayB}
              onPick={() => void vote(matchup.track_b.id)}
            />
          </div>
        ) : (
          <section className="panel p-6 text-center sm:p-8">
            <p className="label">Not ready</p>
            <h2 className="mt-2 font-serif text-3xl">No matchup loaded.</h2>
            <p className="mx-auto mt-2 max-w-md text-sm leading-relaxed text-muted">
              {error ?? 'The admin may not have imported a playlist yet.'}
            </p>
            <button type="button" onClick={() => void load()} className="btn mt-5">
              Try again
            </button>
          </section>
        )}
      </div>
    </>
  )
}

export function VotePage() {
  return (
    <main className="mx-auto w-full max-w-6xl flex-1 px-5 py-4 sm:px-8 sm:py-5">
      <RequireAuth>
        <VoteScreen />
      </RequireAuth>
    </main>
  )
}
