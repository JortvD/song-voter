import { useCallback, useEffect, useRef, useState } from 'react'
import { Link } from 'react-router-dom'

import { Kbd } from '../components/PreviewPlayer'
import { RequireAuth } from '../components/RequireAuth'
import { TapeCard } from '../components/TapeCard'
import { useAuth } from '../context/AuthContext'
import { ApiError, api } from '../lib/api'
import { playStamp } from '../lib/sound'
import type { Matchup } from '../lib/types'

function sleep(milliseconds: number) {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds))
}

function formatNumber(value: number) {
  return value.toLocaleString()
}

function LegendItem({ keys, label }: { keys: string[]; label: string }) {
  return (
    <span className="flex items-center gap-1 font-mono text-[9px] uppercase tracking-[0.12em] text-paper/85">
      {keys.map((key, index) => (
        <span key={key} className="flex items-center gap-1">
          {index > 0 && <span className="text-paper/30">/</span>}
          <Kbd>{key}</Kbd>
        </span>
      ))}
      {label}
    </span>
  )
}

function ShortcutLegend() {
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
      <LegendItem keys={['1']} label="pick A" />
      <LegendItem keys={['2']} label="pick B" />
      <LegendItem keys={['Z']} label="undo" />
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
  const [streak, setStreak] = useState(0)
  const [votesCast, setVotesCast] = useState(0)
  const [totalTracks, setTotalTracks] = useState(0)
  const [tracksSeen, setTracksSeen] = useState(0)
  const [comparedPairs, setComparedPairs] = useState(0)
  const [playlistName, setPlaylistName] = useState('')
  const [playingSide, setPlayingSide] = useState<'a' | 'b' | null>(null)

  const audioA = useRef<HTMLAudioElement | null>(null)
  const audioB = useRef<HTMLAudioElement | null>(null)
  const nextSide = useRef<'a' | 'b'>('a')

  const onPlayA = useCallback((playing: boolean) => {
    if (playing) {
      audioB.current?.pause()
      nextSide.current = 'b'
      setPlayingSide('a')
    } else {
      setPlayingSide((current) => (current === 'a' ? null : current))
    }
  }, [])

  const onPlayB = useCallback((playing: boolean) => {
    if (playing) {
      audioA.current?.pause()
      nextSide.current = 'a'
      setPlayingSide('b')
    } else {
      setPlayingSide((current) => (current === 'b' ? null : current))
    }
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
        setError(caught instanceof ApiError ? caught.message : 'Could not load a duel')
        setMatchup(null)
      }
    } finally {
      setLoading(false)
    }
  }, [applyMatchup])

  useEffect(() => {
    if (user) void load()
  }, [user, load])

  const matchupKey = matchup
    ? `${matchup.track_a.id}:${matchup.track_b.id}`
    : 'none'

  useEffect(() => {
    for (const audio of [audioA.current, audioB.current]) {
      if (!audio) continue
      try {
        audio.pause()
        audio.currentTime = 0
      } catch {
        // The preview may not have loaded metadata yet.
      }
    }
    setPlayingSide(null)
    nextSide.current = 'a'
  }, [matchupKey])

  const playSide = useCallback(
    (side: 'a' | 'b') => {
      if (!matchup) return

      const previewUrl =
        side === 'a' ? matchup.track_a.preview_url : matchup.track_b.preview_url
      const target = side === 'a' ? audioA.current : audioB.current

      if (!previewUrl || !target) {
        setError(`No Spotify preview is available for track ${side.toUpperCase()}.`)
        return
      }

      const other = side === 'a' ? audioB.current : audioA.current
      other?.pause()
      nextSide.current = side === 'a' ? 'b' : 'a'

      void target
        .play()
        .then(() => setError(null))
        .catch(() => setError('Could not play the Spotify preview.'))
    },
    [matchup],
  )

  const playNext = useCallback(() => {
    playSide(nextSide.current)
  }, [playSide])

  const vote = useCallback(
    async (trackId: string) => {
      if (!matchup || voting || undoing) return

      setVoting(true)
      setError(null)
      setPickedId(trackId)
      playStamp()

      try {
        const response = await api.vote({
          track_a: matchup.track_a.id,
          track_b: matchup.track_b.id,
          winner: trackId,
        })

        setVotesCast(response.votes_cast)
        setSessionPicks((value) => value + 1)
        setStreak((value) => value + 1)

        await sleep(320)

        try {
          applyMatchup(await api.getMatchup())
        } catch (caught) {
          if (caught instanceof ApiError && caught.status === 409) {
            setExhausted(true)
            setMatchup(null)
          } else {
            setError(caught instanceof ApiError ? caught.message : 'Could not load the next duel')
            setMatchup(null)
          }
        }
      } catch (caught) {
        setError(caught instanceof ApiError ? caught.message : "That pick didn't save. Try again.")
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
      setStreak(0)
      setComparedPairs(undone.compared_pairs)
      setTracksSeen(undone.tracks_seen)
      setExhausted(false)
      setMatchup({
        playlist_id: undone.playlist_id,
        playlist_name: playlistName || 'Playlist Voter',
        track_a: undone.track_a,
        track_b: undone.track_b,
        compared_pairs: undone.compared_pairs,
        tracks_seen: undone.tracks_seen,
        total_tracks: totalTracks,
        votes_cast: undone.votes_cast,
      })
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : "That undo didn't work. Try again.")
    } finally {
      setUndoing(false)
    }
  }, [undoing, voting, votesCast, playlistName, totalTracks])

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

      if (event.code === 'Space' || event.key === ' ') {
        event.preventDefault()
        playNext()
        return
      }

      const key = event.key.toLowerCase()

      if (key === '1' || event.code === 'Numpad1') {
        event.preventDefault()
        void vote(matchup.track_a.id)
      } else if (key === '2' || event.code === 'Numpad2') {
        event.preventDefault()
        void vote(matchup.track_b.id)
      } else if (key === 'z' && votesCast > 0) {
        event.preventDefault()
        void undo()
      }
    }

    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [matchup, voting, undoing, votesCast, vote, undo, playNext])

  const progress =
    totalTracks > 0 ? Math.min(100, Math.round((tracksSeen / totalTracks) * 100)) : 0

  return (
    <main className="mx-auto w-full max-w-6xl flex-1 px-4 pb-32 pt-4 sm:px-6 sm:pt-5 md:pb-6 lg:px-8">
      <header className="flex flex-wrap items-end justify-between gap-3 border-b-2 border-ink/15 pb-3">
        <div>
          <p className="pv-label">
            Duel
            {playlistName ? ` · now playing from ${playlistName}` : ''}
          </p>
          <h1 className="mt-1 font-serif text-3xl leading-none tracking-[-0.04em] sm:text-4xl">
            Side A or Side B?
          </h1>
          <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {formatNumber(comparedPairs)} {comparedPairs === 1 ? 'duel' : 'duels'} ·{' '}
            {formatNumber(votesCast)} {votesCast === 1 ? 'pick' : 'picks'} ·{' '}
            {formatNumber(sessionPicks)} this session
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Link to="/your-ranking" className="btn-ghost">
            Your Ranking
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
      </header>

      {matchup && totalTracks > 0 && (
        <div className="mt-4">
          <div className="flex items-center justify-between font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            <span>
              {formatNumber(tracksSeen)} of {formatNumber(totalTracks)} songs covered
            </span>
            <span>{progress}%</span>
          </div>
          <div className="mt-1.5 h-2 overflow-hidden rounded-full border-2 border-ink/20 bg-card">
            <div
              className="h-full rounded-full bg-[var(--playlist-accent-strong)] transition-[width] duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
        </div>
      )}

      {error && (
        <p
          role="alert"
          className="mt-4 rounded-xl border-2 border-danger bg-danger/5 px-3 py-2 text-sm text-danger"
        >
          {error}
        </p>
      )}

      {loading ? (
        <div className="mt-4 grid gap-3 lg:grid-cols-[1fr_84px_1fr]">
          <div className="h-96 animate-pulse rounded-[18px] border-2 border-line bg-card" />
          <div className="hidden lg:block" />
          <div className="h-96 animate-pulse rounded-[18px] border-2 border-line bg-card" />
        </div>
      ) : exhausted ? (
        <section className="pv-card mt-4 p-6 text-center sm:p-10">
          <span className="pv-sticker">All duels done</span>
          <h2 className="mt-4 font-serif text-4xl leading-none tracking-[-0.03em]">
            You&apos;ve heard every side.
          </h2>
          <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
            Your tape has {formatNumber(votesCast)} {votesCast === 1 ? 'pick' : 'picks'}.
            The standings stay sealed until The Reveal.
          </p>
          <div className="mt-6 flex flex-wrap justify-center gap-3">
            <Link to="/your-ranking" className="btn">
              Open your ranking
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
        <>
          <div className="pv-deck mt-4">
            <div className="pv-deck-top">
              <span>Cassette deck · Side A / Side B</span>
              <span>Round {String(sessionPicks + 1).padStart(2, '0')}</span>
            </div>

            <div className="grid items-stretch gap-3 lg:grid-cols-[1fr_84px_1fr]">
              <TapeCard
                side="a"
                track={matchup.track_a}
                picked={pickedId === matchup.track_a.id}
                playing={playingSide === 'a'}
                disabled={voting || undoing}
                audioRef={audioA}
                onPlayStateChange={onPlayA}
                onPick={() => void vote(matchup.track_a.id)}
              />

              <div className="flex items-center justify-center py-1">
                <span className="pv-vs">VS</span>
              </div>

              <TapeCard
                side="b"
                track={matchup.track_b}
                picked={pickedId === matchup.track_b.id}
                playing={playingSide === 'b'}
                disabled={voting || undoing}
                audioRef={audioB}
                onPlayStateChange={onPlayB}
                onPick={() => void vote(matchup.track_b.id)}
              />
            </div>

            <div className="mt-3 flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-paper/20 bg-paper/5 px-3 py-2.5">
              <div className="flex items-center gap-2">
                {streak > 1 ? (
                  <span className="pv-sticker px-1.5 py-0.5 text-[9px]">
                    {streak} in a row
                  </span>
                ) : (
                  <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-paper/80">
                    Tap a side or press Space
                  </span>
                )}
              </div>

              <div className="flex flex-wrap items-center gap-2">
                <button
                  type="button"
                  onClick={() => playSide('a')}
                  className={`pv-transport hidden sm:inline-flex ${
                    playingSide === 'a' ? 'pv-transport-active' : ''
                  }`}
                  aria-label="Play side A"
                >
                  Play A
                </button>
                <button type="button" onClick={playNext} className="pv-transport">
                  <Kbd>Space</Kbd> play / switch
                </button>
                <button
                  type="button"
                  onClick={() => playSide('b')}
                  className={`pv-transport hidden sm:inline-flex ${
                    playingSide === 'b' ? 'pv-transport-active' : ''
                  }`}
                  aria-label="Play side B"
                >
                  Play B
                </button>
              </div>

              <ShortcutLegend />
            </div>
          </div>

          <div className="fixed inset-x-0 bottom-[4.6rem] z-30 px-3 md:hidden">
            <div className="mx-auto flex max-w-md gap-2 rounded-2xl border-2 border-ink bg-card p-2 shadow-[4px_4px_0_0_#141a2a]">
              <button
                type="button"
                onClick={() => void vote(matchup.track_a.id)}
                disabled={voting || undoing}
                className="flex flex-1 items-center justify-center gap-1.5 rounded-xl border-2 border-ink bg-[var(--playlist-accent)] px-3 py-2.5 font-mono text-[10px] font-semibold uppercase tracking-[0.12em] text-[var(--playlist-accent-ink)] disabled:opacity-50"
              >
                Pick A <Kbd>1</Kbd>
              </button>
              <button
                type="button"
                onClick={() => void vote(matchup.track_b.id)}
                disabled={voting || undoing}
                className="flex flex-1 items-center justify-center gap-1.5 rounded-xl border-2 border-ink bg-[var(--playlist-accent)] px-3 py-2.5 font-mono text-[10px] font-semibold uppercase tracking-[0.12em] text-[var(--playlist-accent-ink)] disabled:opacity-50"
              >
                Pick B <Kbd>2</Kbd>
              </button>
            </div>
          </div>
        </>
      ) : (
        <section className="pv-card mt-4 p-6 text-center sm:p-8">
          <span className="pv-sticker pv-sticker-paper">Not ready</span>
          <h2 className="mt-4 font-serif text-3xl">This duel didn't load.</h2>
          <p className="mx-auto mt-2 max-w-md text-sm leading-relaxed text-muted">
            {error ?? 'The admin may not have imported a playlist yet.'}
          </p>
          <button type="button" onClick={() => void load()} className="btn mt-5">
            Try again
          </button>
        </section>
      )}
    </main>
  )
}

export function VotePage() {
  return (
    <RequireAuth>
      <VoteScreen />
    </RequireAuth>
  )
}
