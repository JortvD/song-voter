import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { Link } from 'react-router-dom'

import { Kbd } from '../components/Kbd'
import { RequireAuth } from '../components/RequireAuth'
import { SeasonUnavailable } from '../components/SeasonUnavailable'
import { AutoplayToggle } from '../components/duel/AutoplayToggle'
import { CassetteHub } from '../components/duel/CassetteHub'
import { DuelCassette } from '../components/duel/DuelCassette'
import { DuelStatus } from '../components/duel/DuelStatus'
import { RecentDuels } from '../components/duel/RecentDuels'
import { StreakBar } from '../components/duel/StreakBar'
import { useAuth } from '../context/AuthContext'
import { usePlaylist } from '../context/PlaylistContext'
import { ApiError, api } from '../lib/api'
import {
  playPickConfirm,
  playPickPress,
  playStreakGoal,
  playStreakMilestone,
  playToggle,
  playUndo,
} from '../lib/sound'
import { localTimezone } from '../lib/time'
import type { DuelHistoryItem, Matchup, StreakSummary } from '../lib/types'

const AUTOPREVIEW_KEY = 'pv-autoplay-previews'
const CROSSFADE_MS = 200

function initialAutoPreview(): boolean {
  try {
    return window.localStorage.getItem(AUTOPREVIEW_KEY) === 'on'
  } catch {
    return false
  }
}

function formatWhen(value: string | null) {
  if (!value) return 'soon'
  return new Date(value).toLocaleString(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  })
}

function sleep(milliseconds: number) {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds))
}

function storedDuelKey(userId: string) {
  return `pv-current-duel:${userId}`
}

function readStoredDuel(userId: string, playlistId: string): Matchup | null {
  try {
    const raw = window.sessionStorage.getItem(storedDuelKey(userId))
    if (!raw) return null
    const value = JSON.parse(raw) as Matchup
    if (
      value?.playlist_id === playlistId &&
      value.track_a?.id &&
      value.track_b?.id
    ) {
      return value
    }
  } catch {
    // Storage can be disabled or contain an old/corrupt value.
  }
  return null
}

function smallViewportHeight() {
  if (typeof CSS === 'undefined' || !CSS.supports('height', '100svh')) {
    return window.innerHeight
  }

  const probe = document.createElement('div')
  probe.style.cssText =
    'position:fixed;top:0;left:0;width:0;height:100svh;visibility:hidden;pointer-events:none'
  document.body.appendChild(probe)
  const height = probe.getBoundingClientRect().height
  probe.remove()
  return height
}

function vibrate() {
  const nav = navigator as Navigator & {
    vibrate?: (pattern: number | number[]) => boolean
  }
  nav.vibrate?.(12)
}

function DuelSkeletonSide() {
  return (
    <div className="pv-duel-side">
      <div className="pv-duel-skel-cover" />
      <div className="pv-duel-skel-copy">
        <span className="pv-duel-skel-line w-14" />
        <span className="pv-duel-skel-line h-4 w-4/5" />
        <span className="pv-duel-skel-line h-2.5 w-2/3" />
        <span className="pv-duel-skel-line h-3 w-16" />
      </div>
      <div className="pv-duel-skel-listen" />
    </div>
  )
}

function DuelSkeleton() {
  return (
    <div className="pv-duel-cassette pv-duel-skeleton" aria-hidden="true">
      <div className="pv-duel-sides">
        <DuelSkeletonSide />
        <CassetteHub playing={false} />
        <DuelSkeletonSide />
      </div>
    </div>
  )
}

function VoteScreen() {
  const { user } = useAuth()
  const {
    status,
    loading: playlistLoading,
    error: playlistError,
    refresh: refreshPlaylist,
  } = usePlaylist()
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
  const [playingSide, setPlayingSide] = useState<'a' | 'b' | null>(null)
  const [recentDuels, setRecentDuels] = useState<DuelHistoryItem[]>([])
  const [recentHeight, setRecentHeight] = useState<number | null>(null)
  const [autoPreview, setAutoPreview] = useState(initialAutoPreview)
  const [autoDockVisible, setAutoDockVisible] = useState(false)

  const stageRef = useRef<HTMLDivElement | null>(null)
  const cardRef = useRef<HTMLDivElement | null>(null)
  const autoDockRef = useRef<HTMLDivElement | null>(null)
  const audioA = useRef<HTMLAudioElement | null>(null)
  const audioB = useRef<HTMLAudioElement | null>(null)
  const nextSide = useRef<'a' | 'b'>('a')
  const autoPreviewRef = useRef(autoPreview)
  const fadeFrames = useRef<Record<'a' | 'b', number | null>>({
    a: null,
    b: null,
  })
  const previousStreak = useRef<StreakSummary | null>(null)
  const focusNextPickRef = useRef(false)
  const exhaustedTitleRef = useRef<HTMLHeadingElement | null>(null)
  const storageKey = user ? storedDuelKey(user.id) : null

  useEffect(() => {
    if (!storageKey || !matchup) return
    try {
      window.sessionStorage.setItem(storageKey, JSON.stringify(matchup))
    } catch {
      // Storage can be disabled; the in-memory duel still works.
    }
  }, [storageKey, matchup])

  useEffect(() => {
    if (!storageKey || !exhausted) return
    try {
      window.sessionStorage.removeItem(storageKey)
    } catch {
      // Ignore storage failures.
    }
  }, [storageKey, exhausted])

  useEffect(() => {
    autoPreviewRef.current = autoPreview
    try {
      window.localStorage.setItem(AUTOPREVIEW_KEY, autoPreview ? 'on' : 'off')
    } catch {
      // Storage can be disabled; the in-memory preference still works.
    }
  }, [autoPreview])

  useEffect(() => {
    let frame = 0
    let cancelled = false

    const update = () => {
      frame = 0
      if (cancelled) return
      const dock = autoDockRef.current
      if (!dock) return

      const tabs = document.querySelector<HTMLElement>(
        'nav[aria-label="Primary"]',
      )
      const bottom = tabs
        ? tabs.getBoundingClientRect().top
        : window.innerHeight
      setAutoDockVisible(dock.getBoundingClientRect().bottom <= bottom - 4)
    }

    const schedule = () => {
      if (frame) return
      frame = window.requestAnimationFrame(update)
    }

    update()
    void document.fonts?.ready.then(schedule)
    window.addEventListener('scroll', schedule, { passive: true })
    window.addEventListener('resize', schedule)
    return () => {
      cancelled = true
      if (frame) window.cancelAnimationFrame(frame)
      window.removeEventListener('scroll', schedule)
      window.removeEventListener('resize', schedule)
    }
  }, [matchup, recentHeight, recentDuels.length, votesCast])

  const cancelFade = useCallback((side: 'a' | 'b') => {
    const frame = fadeFrames.current[side]
    if (frame !== null) cancelAnimationFrame(frame)
    fadeFrames.current[side] = null
  }, [])

  const fadeVolume = useCallback(
    (
      side: 'a' | 'b',
      targetVolume: number,
      duration: number,
      pauseAtEnd = false,
    ) => {
      const audio = side === 'a' ? audioA.current : audioB.current
      if (!audio) return

      cancelFade(side)
      const from = audio.volume
      const started = performance.now()

      const step = (now: number) => {
        const progress = Math.min(1, (now - started) / Math.max(1, duration))
        audio.volume = Math.max(
          0,
          Math.min(1, from + (targetVolume - from) * progress),
        )

        if (progress < 1) {
          fadeFrames.current[side] = requestAnimationFrame(step)
        } else {
          fadeFrames.current[side] = null
          if (pauseAtEnd) audio.pause()
        }
      }

      fadeFrames.current[side] = requestAnimationFrame(step)
    },
    [cancelFade],
  )

  const pauseAll = useCallback(() => {
    cancelFade('a')
    cancelFade('b')

    for (const audio of [audioA.current, audioB.current]) {
      try {
        audio?.pause()
        if (audio) audio.volume = 1
      } catch {
        // Ignore teardown races.
      }
    }
    setPlayingSide(null)
  }, [cancelFade])

  const onPlayA = useCallback((playing: boolean) => {
    if (playing) {
      nextSide.current = 'b'
      setPlayingSide('a')
    } else {
      setPlayingSide((current) => (current === 'a' ? null : current))
    }
  }, [])

  const onPlayB = useCallback((playing: boolean) => {
    if (playing) {
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
    setRecentDuels(next.recent_duels ?? [])
    setExhausted(false)
    setError(null)
    setPickedId(null)
  }, [])

  const votingBlocked = Boolean(
    status?.playlist_name &&
      (status.started_at === null || status.voting_paused),
  )

  const load = useCallback(
    async (signal?: AbortSignal) => {
      if (playlistLoading || !status || votingBlocked) {
        setLoading(false)
        return
      }

      setLoading(true)
      setError(null)

      try {
        applyMatchup(await api.getMatchup(localTimezone(), signal))
      } catch (caught) {
        if (signal?.aborted) return
        if (caught instanceof ApiError && caught.status === 409) {
          setExhausted(true)
          setMatchup(null)
        } else {
          setError(
            caught instanceof ApiError ? caught.message : 'Could not load a duel',
          )
          setMatchup(null)
        }
      } finally {
        if (!signal?.aborted) setLoading(false)
      }
    },
    [applyMatchup, playlistLoading, status?.playlist_id, votingBlocked],
  )

  useEffect(() => {
    if (!user) return

    const stored =
      status?.playlist_id != null
        ? readStoredDuel(user.id, status.playlist_id)
        : null
    if (stored) {
      applyMatchup(stored)
      setLoading(false)
      return
    }

    const controller = new AbortController()
    void load(controller.signal)
    return () => controller.abort()
  }, [user, load, status?.playlist_id, applyMatchup])

  const matchupKey = matchup
    ? `${matchup.track_a.id}:${matchup.track_b.id}`
    : 'none'

  useEffect(() => {
    cancelFade('a')
    cancelFade('b')

    for (const audio of [audioA.current, audioB.current]) {
      if (!audio) continue
      try {
        audio.pause()
        audio.currentTime = 0
        audio.volume = 1
      } catch {
        // The preview may not have loaded metadata yet.
      }
    }
    setPlayingSide(null)
    nextSide.current = 'a'
  }, [matchupKey, cancelFade])

  useEffect(() => {
    if (!focusNextPickRef.current || voting || undoing) return
    if (!matchup) return

    focusNextPickRef.current = false
    const frame = window.requestAnimationFrame(() => {
      document
        .querySelector<HTMLButtonElement>('[data-duel-pick="a"]')
        ?.focus()
    })

    return () => window.cancelAnimationFrame(frame)
  }, [matchupKey, matchup, voting, undoing])

  useEffect(() => {
    if (!exhausted) return
    focusNextPickRef.current = false
    const frame = window.requestAnimationFrame(() => {
      exhaustedTitleRef.current?.focus()
    })
    return () => window.cancelAnimationFrame(frame)
  }, [exhausted])

  useLayoutEffect(() => {
    if (!matchup) {
      setRecentHeight(null)
      return
    }

    let cancelled = false

    const measure = () => {
      if (cancelled) return

      const stage = stageRef.current
      const card = cardRef.current
      if (!stage || !card) return

      const tabs = document.querySelector<HTMLElement>('nav[aria-label="Primary"]')
      const viewportHeight = smallViewportHeight()
      const scroll = window.scrollY
      // Work entirely in document space so scrolling/collapsing browser
      // chrome cannot resize or shift the recent-duels panel.
      const bottom = scroll + viewportHeight - (tabs?.offsetHeight ?? 64)
      const stageTop = stage.getBoundingClientRect().top + scroll
      const available = bottom - stageTop - card.offsetHeight - 12 - 8

      // Apply the same cap at every width, including desktop/tablet where an
      // uncapped panel pushed the duel controls below the fold.
      setRecentHeight(Math.min(Math.max(Math.floor(available), 0), 190))
    }
    measure()
    const observer =
      typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(measure)
    if (observer && cardRef.current) observer.observe(cardRef.current)
    window.addEventListener('resize', measure)
    void document.fonts?.ready.then(measure)

    return () => {
      cancelled = true
      observer?.disconnect()
      window.removeEventListener('resize', measure)
    }
  }, [matchup, error, votesCast, recentDuels.length])

  const startSide = useCallback(
    (side: 'a' | 'b', options: { auto?: boolean } = {}) => {
      if (!matchup) return false

      const previewUrl =
        side === 'a' ? matchup.track_a.preview_url : matchup.track_b.preview_url
      const target = side === 'a' ? audioA.current : audioB.current

      if (!previewUrl || !target) {
        if (!options.auto) {
          setError(
            `No Spotify preview is available for track ${side.toUpperCase()}.`,
          )
        }
        return false
      }

      const otherSide = side === 'a' ? 'b' : 'a'
      const other = otherSide === 'a' ? audioA.current : audioB.current
      nextSide.current = otherSide

      if (!target.paused) {
        fadeVolume(side, 1, CROSSFADE_MS)
        if (other && !other.paused) fadeVolume(otherSide, 0, CROSSFADE_MS, true)
        return true
      }

      const otherPlaying = Boolean(other && !other.paused)
      const shouldFade = otherPlaying || options.auto === true
      target.volume = shouldFade ? 0 : 1

      void target
        .play()
        .then(() => {
          setError(null)
          if (shouldFade) fadeVolume(side, 1, CROSSFADE_MS)
          if (otherPlaying) fadeVolume(otherSide, 0, CROSSFADE_MS, true)
        })
        .catch(() => {
          target.volume = 1
          setPlayingSide((current) => (current === side ? null : current))
          if (!options.auto) setError('Could not play the Spotify preview.')
        })

      return true
    },
    [matchup, fadeVolume],
  )

  const toggleSide = useCallback(
    (side: 'a' | 'b') => {
      if (!matchup || voting || undoing) return

      const previewUrl =
        side === 'a' ? matchup.track_a.preview_url : matchup.track_b.preview_url
      const target = side === 'a' ? audioA.current : audioB.current

      if (!previewUrl || !target) {
        setError(`No Spotify preview is available for track ${side.toUpperCase()}.`)
        return
      }

      if (!target.paused) {
        fadeVolume(side, 0, 160, true)
        setPlayingSide(null)
        return
      }

      startSide(side)
    },
    [matchup, voting, undoing, fadeVolume, startSide],
  )

  const playNext = useCallback(() => {
    startSide(nextSide.current)
  }, [startSide])

  const handleEnded = useCallback(
    (side: 'a' | 'b') => {
      setPlayingSide(null)

      if (!autoPreviewRef.current) return
      if (side !== 'a') return

      const started = startSide('b', { auto: true })
      if (!started) setPlayingSide(null)
    },
    [startSide],
  )

  const toggleAutoPreview = useCallback(() => {
    const next = !autoPreviewRef.current
    autoPreviewRef.current = next
    setAutoPreview(next)
    playToggle(next)

    if (next) {
      const alreadyPlaying =
        (audioA.current && !audioA.current.paused) ||
        (audioB.current && !audioB.current.paused)

      if (!alreadyPlaying) {
        const started = startSide('a', { auto: true })
        if (!started) startSide('b', { auto: true })
      }
    }
  }, [startSide])

  const startSideRef = useRef(startSide)
  startSideRef.current = startSide

  useEffect(() => {
    if (!autoPreviewRef.current || !matchup) return

    const started = startSideRef.current('a', { auto: true })
    if (!started) startSideRef.current('b', { auto: true })
  }, [matchupKey])

  useEffect(() => {
    const current = matchup?.streak ?? null
    const previous = previousStreak.current

    if (!current) {
      previousStreak.current = null
      return
    }

    if (previous && !previous.today_met && current.today_met) {
      if ([7, 30, 100].includes(current.current)) {
        playStreakMilestone()
      } else {
        playStreakGoal()
      }
    }

    previousStreak.current = current
  }, [matchup?.streak])

  const vote = useCallback(
    async (trackId: string) => {
      if (!matchup || voting || undoing) return

      setVoting(true)
      setError(null)
      setPickedId(trackId)
      pauseAll()
      const pickedSide = trackId === matchup.track_a.id ? 'a' : 'b'
      playPickPress(pickedSide)
      vibrate()

      try {
        const response = await api.vote({
          track_a: matchup.track_a.id,
          track_b: matchup.track_b.id,
          winner: trackId,
        })

        playPickConfirm(pickedSide)
        setVotesCast(response.votes_cast)
        setSessionPicks((value) => value + 1)

        // Start fetching the next duel immediately, but keep the stamp visible
        // for a beat so rapid picks still feel like picks.
        const [nextResult] = await Promise.allSettled([
          api.getMatchup(localTimezone()),
          sleep(280),
        ])

        if (nextResult.status === 'fulfilled') {
          focusNextPickRef.current = true
          applyMatchup(nextResult.value)
        } else if (
          nextResult.reason instanceof ApiError &&
          nextResult.reason.status === 409
        ) {
          focusNextPickRef.current = true
          setExhausted(true)
          setMatchup(null)
        } else {
          focusNextPickRef.current = false
          setError("Your pick saved, but the next duel didn't load. Try again.")
          setMatchup(null)
        }
      } catch (caught) {
        focusNextPickRef.current = false
        setError(
          caught instanceof ApiError ? caught.message : "That pick didn't save. Try again.",
        )
      } finally {
        setVoting(false)
        setPickedId(null)
      }
    },
    [matchup, voting, undoing, pauseAll, applyMatchup],
  )

  const undo = useCallback(async () => {
    if (undoing || voting || votesCast === 0) return

    setUndoing(true)
    setError(null)
    pauseAll()

    try {
      const undone = await api.undoVote(localTimezone())
      playUndo()

      setVotesCast(undone.votes_cast)
      setSessionPicks((value) => Math.max(0, value - 1))
      setComparedPairs(undone.compared_pairs)
      setTracksSeen(undone.tracks_seen)
      setExhausted(false)
      setPickedId(null)
      setRecentDuels(undone.recent_duels ?? [])
      focusNextPickRef.current = true
      setMatchup({
        playlist_id: undone.playlist_id,
        playlist_name: playlistName || 'Playlist Voter',
        track_a: undone.track_a,
        track_b: undone.track_b,
        compared_pairs: undone.compared_pairs,
        tracks_seen: undone.tracks_seen,
        total_tracks: totalTracks,
        votes_cast: undone.votes_cast,
        recent_duels: undone.recent_duels ?? [],
        streak: undone.streak,
      })
    } catch (caught) {
      setError(
        caught instanceof ApiError ? caught.message : "That undo didn't work. Try again.",
      )
    } finally {
      setUndoing(false)
    }
  }, [undoing, voting, votesCast, pauseAll, playlistName, totalTracks])

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
        // Space is always the preview shortcut. A pick control can keep focus
        // after a vote, so do not let Space activate it as a second pick.
        if (!target?.closest('[data-duel-pick]')) {
          // Let other focused controls handle their own Space activation.
          if (target?.closest('button, a, [role="button"]')) return
        }
        event.preventDefault()
        playNext()
        return
      }

      const key = event.key.toLowerCase()

      if (
        key === '1' ||
        event.code === 'Numpad1' ||
        key === 'a' ||
        event.key === 'ArrowLeft'
      ) {
        event.preventDefault()
        void vote(matchup.track_a.id)
      } else if (
        key === '2' ||
        event.code === 'Numpad2' ||
        key === 'b' ||
        event.key === 'ArrowRight'
      ) {
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

  const busy = voting || undoing

  if (playlistLoading) {
    return (
      <main className="pv-duel-page mx-auto flex w-full max-w-6xl flex-1 flex-col px-3 pt-3 sm:px-6 sm:pt-4 lg:px-8">
        <div className="my-auto">
          <DuelSkeleton />
        </div>
      </main>
    )
  }

  if (!status) {
    return (
      <main className="pv-duel-page mx-auto flex w-full max-w-6xl flex-1 flex-col px-3 pt-3 sm:px-6 sm:pt-4 lg:px-8">
        <div className="my-auto">
          {playlistError ? (
            <SeasonUnavailable
              error={playlistError}
              onRetry={refreshPlaylist}
              title="The duel is unavailable."
            />
          ) : (
            <DuelSkeleton />
          )}
        </div>
      </main>
    )
  }

  if (votingBlocked) {
    const scheduled = status.started_at === null

    return (
      <main className="pv-duel-page mx-auto flex w-full max-w-6xl flex-1 flex-col px-3 pt-3 sm:px-6 sm:pt-4 lg:px-8">
        <section className="pv-duel-empty my-auto">
          <span
            className={`pv-sticker ${
              scheduled ? 'pv-sticker-butter' : 'pv-sticker-paper'
            }`}
          >
            {scheduled ? 'Scheduled' : 'Voting paused'}
          </span>
          <h1 className="mt-3 font-serif text-3xl leading-none tracking-[-0.03em] sm:text-4xl">
            {scheduled ? 'The tape isn’t open yet.' : 'Voting is paused.'}
          </h1>
          <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
            {scheduled
              ? `Voting opens ${formatWhen(
                  status.scheduled_start_at,
                )}. You can log in now; duels unlock then.`
              : 'The admin has paused new picks. Existing picks and rankings are safe.'}
          </p>
          <div className="mt-5 flex flex-wrap justify-center gap-3">
            <Link to="/standings" className="btn-ghost">
              The Standings
            </Link>
            <Link to="/stats" className="btn-ghost">
              The Stats
            </Link>
            <Link to="/" className="btn-ghost">
              Home
            </Link>
          </div>
        </section>
      </main>
    )
  }

  return (
    <main className="pv-duel-page mx-auto flex w-full max-w-6xl flex-1 flex-col px-3 pt-3 sm:px-6 sm:pt-4 lg:px-8">
      <DuelStatus
        round={sessionPicks + 1}
        comparedPairs={comparedPairs}
        votesCast={votesCast}
        tracksSeen={tracksSeen}
        totalTracks={totalTracks}
        undoing={undoing}
        disabled={busy}
        onUndo={() => void undo()}
      />

      {matchup && <StreakBar streak={matchup.streak} />}

      <p className="sr-only" aria-live="polite">
        {pickedId && matchup
          ? `Picked ${
              pickedId === matchup.track_a.id
                ? matchup.track_a.title
                : matchup.track_b.title
            }`
          : ''}
      </p>

      {error && (matchup || exhausted) && (
        <p role="alert" className="pv-duel-error">
          {error}
        </p>
      )}

      <div className="flex min-h-0 flex-1 flex-col py-3">
        <div className="flex min-h-0 flex-1 flex-col">
          {loading ? (
            <div className="my-auto">
              <DuelSkeleton />
            </div>
          ) : exhausted ? (
            <section className="pv-duel-empty my-auto">
              <span className="pv-sticker">All duels done</span>
              <h1
                ref={exhaustedTitleRef}
                tabIndex={-1}
                className="mt-3 font-serif text-3xl leading-none tracking-[-0.03em] outline-none sm:text-4xl"
              >
                You&apos;ve heard every side.
              </h1>
              <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
                Your tape has {votesCast.toLocaleString()}{' '}
                {votesCast === 1 ? 'pick' : 'picks'}. The standings stay sealed
                until The Reveal.
              </p>
              <div className="mt-5 flex flex-wrap justify-center gap-3">
                <Link to="/your-ranking" className="btn">
                  Open your ranking
                </Link>
                <Link to="/your-duels" className="btn-ghost">
                  See all your duels
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
            <div
              ref={stageRef}
              className="flex min-h-0 flex-1 flex-col gap-3"
            >
              <h1 className="sr-only">Duel: pick the better side</h1>

              <RecentDuels
                duels={recentDuels}
                height={recentHeight}
                className={`mx-auto min-h-0 w-full max-w-5xl ${
                  recentHeight == null ? 'flex-1' : 'shrink-0'
                }`}
              />

              <div
                ref={cardRef}
                className="relative mx-auto mt-auto flex w-full max-w-5xl shrink-0 flex-col gap-3"
              >
                <DuelCassette
                  matchup={matchup}
                  pickedId={pickedId}
                  playingSide={playingSide}
                  disabled={busy}
                  audioA={audioA}
                  audioB={audioB}
                  onPlayA={onPlayA}
                  onPlayB={onPlayB}
                  onTogglePreview={toggleSide}
                  onEnded={handleEnded}
                  onPick={(side) => {
                    const trackId =
                      side === 'a' ? matchup.track_a.id : matchup.track_b.id
                    void vote(trackId)
                  }}
                />

                <div
                  ref={autoDockRef}
                  className={`pv-autoplay-dock ${
                    autoDockVisible ? 'is-visible' : ''
                  }`}
                >
                  <AutoplayToggle
                    enabled={autoPreview}
                    onToggle={toggleAutoPreview}
                    disabled={busy}
                  />
                </div>

                <p className="pv-duel-hint hidden lg:flex">
                  <Kbd>1</Kbd>
                  <span>/</span>
                  <Kbd>2</Kbd>
                  <span>pick</span>
                  <span className="text-ink/25">·</span>
                  <Kbd>←</Kbd>
                  <span>/</span>
                  <Kbd>→</Kbd>
                  <span>swipe</span>
                  <span className="text-ink/25">·</span>
                  <Kbd>Space</Kbd>
                  <span>preview</span>
                  <span className="text-ink/25">·</span>
                  <Kbd>Z</Kbd>
                  <span>undo</span>
                </p>
              </div>
            </div>
          ) : (
            <section className="pv-duel-empty my-auto">
              <span className="pv-sticker pv-sticker-paper">Not ready</span>
              <h1 className="mt-3 font-serif text-3xl leading-none tracking-[-0.03em]">
                This duel didn&apos;t load.
              </h1>
              <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
                {error ?? 'The admin may not have imported a playlist yet.'}
              </p>
              <button
                type="button"
                onClick={() => void load()}
                className="btn mt-5"
              >
                Try again
              </button>
            </section>
          )}
        </div>
      </div>
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
