import { useEffect, useState } from 'react'

import { usePlaylist } from '../context/PlaylistContext'
import { ApiError, api } from '../lib/api'
import type { ChartStatus } from '../lib/types'

function formatNumber(value: number) {
  return value.toLocaleString()
}

function formatDateTime(value: string | null) {
  if (!value) return '—'
  return new Date(value).toLocaleString(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  })
}

function localInputValue(value: string | null) {
  if (!value) return ''
  const date = new Date(value)
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000)
  return local.toISOString().slice(0, 16)
}

export function AdminReveal({
  onOpenPlaylist,
}: {
  onOpenPlaylist?: () => void
}) {
  const { refresh: refreshPlaylist } = usePlaylist()
  const [status, setStatus] = useState<ChartStatus | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [revealInput, setRevealInput] = useState('')
  const [pauseOnReveal, setPauseOnReveal] = useState(false)
  const hasTape = Boolean(status?.playlist_id)

  useEffect(() => {
    let active = true

    api
      .getChartStatus()
      .then((next) => {
        if (!active) return
        setStatus(next)
        setRevealInput(localInputValue(next.scheduled_reveal_at))
        setPauseOnReveal(next.pause_voting_on_reveal)
      })
      .catch((caught: unknown) => {
        if (active) {
          setError(caught instanceof ApiError ? caught.message : 'Could not load The Reveal status')
        }
      })

    return () => {
      active = false
    }
  }, [])

  useEffect(() => {
    const timer = window.setInterval(() => {
      api
        .getChartStatus()
        .then((next) => setStatus(next))
        .catch(() => {
          // Keep the last known state; the manual refresh still works.
        })
    }, 15_000)

    return () => window.clearInterval(timer)
  }, [])

  async function toggleReveal() {
    if (!status || !hasTape) return

    const next = !status.revealed
    if (
      next &&
      !window.confirm(
        'Open The Reveal to everyone? Picking stays open and you can seal the standings again.',
      )
    ) {
      return
    }

    setBusy(true)
    setError(null)

    try {
      setStatus(await api.setReveal(next))
      await refreshPlaylist()
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not change The Reveal')
    } finally {
      setBusy(false)
    }
  }

  async function scheduleReveal() {
    if (!status?.playlist_id || !revealInput || busy) return
    const revealAt = new Date(revealInput)
    if (Number.isNaN(revealAt.getTime())) {
      setError('Choose a valid reveal date and time')
      return
    }

    setBusy(true)
    setError(null)
    setNotice(null)

    try {
      const next = await api.scheduleReveal(revealAt.toISOString(), pauseOnReveal)
      setStatus(next)
      await refreshPlaylist()
      setNotice(
        pauseOnReveal
          ? 'Reveal scheduled. Voting will pause when it opens.'
          : 'Reveal scheduled.',
      )
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : 'Could not schedule The Reveal',
      )
    } finally {
      setBusy(false)
    }
  }

  async function cancelReveal() {
    if (!status?.playlist_id || busy) return
    setBusy(true)
    setError(null)
    setNotice(null)

    try {
      const next = await api.cancelScheduledReveal()
      setStatus(next)
      await refreshPlaylist()
      setRevealInput('')
      setPauseOnReveal(false)
      setNotice('Scheduled reveal cancelled.')
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : 'Could not cancel the scheduled reveal',
      )
    } finally {
      setBusy(false)
    }
  }

  async function togglePause() {
    if (!status?.playlist_id || busy) return
    setBusy(true)
    setError(null)
    setNotice(null)

    try {
      await api.setVotingPaused(!status.voting_paused)
      const next = await api.getChartStatus()
      setStatus(next)
      await refreshPlaylist()
      setNotice(next.voting_paused ? 'Voting paused.' : 'Voting resumed.')
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : 'Could not change the voting pause',
      )
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className="panel">
      <div className="flex items-center justify-between border-b border-line px-5 py-3">
        <h2 className="label">The Reveal</h2>
        <span
          className={`border px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-[0.14em] ${
            status?.revealed
              ? 'border-ok/40 text-ok'
              : 'border-line text-muted'
          }`}
        >
          {!status
            ? 'Loading'
            : !hasTape
              ? 'No tape'
              : status.revealed
                ? 'Live'
                : 'Sealed'}
        </span>
      </div>

      <div className="p-5">
        <p className="text-sm leading-relaxed text-muted">
          The shared standings stay hidden from voters until The Reveal.
          Picking continues afterwards, and you can seal them again.
        </p>

        {status && !hasTape && (
          <div className="mt-4 border border-dashed border-line bg-paper px-4 py-4">
            <p className="font-medium text-ink">No tape loaded.</p>
            <p className="mt-1 text-sm leading-relaxed text-muted">
              Import a Spotify playlist before opening or scheduling The Reveal.
            </p>
            {onOpenPlaylist && (
              <button
                type="button"
                onClick={onOpenPlaylist}
                className="btn-ghost mt-3"
              >
                Open Playlist
              </button>
            )}
          </div>
        )}

        {status && hasTape && (
          <p className="mt-3 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {formatNumber(status.total_tracks)} songs ·{' '}
            {formatNumber(status.votes_cast)} picks
          </p>
        )}

        {status && hasTape && (
          <div className="mt-4 border border-line bg-paper p-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <p className="label">Voting</p>
                <p className="mt-1 text-sm">
                  {status.started_at === null
                    ? 'Waiting for the tape start'
                    : status.voting_paused
                      ? 'Paused'
                      : 'Open'}
                </p>
              </div>
              <button
                type="button"
                onClick={() => void togglePause()}
                disabled={busy}
                className="btn-ghost shrink-0"
              >
                {status.voting_paused ? 'Resume voting' : 'Pause voting'}
              </button>
            </div>
          </div>
        )}

        {status && hasTape && !status.revealed && (
          <div className="mt-3 border border-line bg-paper p-3">
            <p className="label">Scheduled reveal</p>
            <p className="mt-1 text-sm text-muted">
              {status.scheduled_reveal_at
                ? `Scheduled for ${formatDateTime(status.scheduled_reveal_at)}${
                    status.pause_voting_on_reveal ? ' · voting will pause' : ''
                  }`
                : 'No reveal scheduled.'}
            </p>

            <div className="mt-3 grid gap-2 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
              <label className="block min-w-0">
                <span className="label mb-1 block">Open at</span>
                <input
                  type="datetime-local"
                  value={revealInput}
                  onChange={(event) => setRevealInput(event.target.value)}
                  className="field min-w-0"
                />
              </label>
              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  onClick={() => void scheduleReveal()}
                  disabled={busy || revealInput.length === 0}
                  className="btn"
                >
                  {status.scheduled_reveal_at ? 'Update reveal' : 'Schedule reveal'}
                </button>
                {status.scheduled_reveal_at && (
                  <button
                    type="button"
                    onClick={() => void cancelReveal()}
                    disabled={busy}
                    className="btn-ghost"
                  >
                    Cancel
                  </button>
                )}
              </div>
            </div>

            <label className="mt-3 flex items-center gap-2 text-sm text-muted">
              <input
                type="checkbox"
                checked={pauseOnReveal}
                onChange={(event) => setPauseOnReveal(event.target.checked)}
              />
              Pause voting when the reveal opens
            </label>
          </div>
        )}

        {notice && (
          <p className="mt-3 border-l-2 border-ok bg-ok/5 px-3 py-2 text-xs text-ok">
            {notice}
          </p>
        )}

        {error && (
          <p className="mt-3 border-l-2 border-danger bg-danger/5 px-3 py-2 text-xs text-danger">
            {error}
          </p>
        )}

        {hasTape && (
          <div className="mt-4 flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => void toggleReveal()}
              disabled={busy || !status}
              className="btn"
            >
              {busy ? 'Working…' : status?.revealed ? 'Seal the standings' : 'Open The Reveal'}
            </button>
            <a href="/api/admin/chart.csv" className="btn-ghost">
              Download CSV
            </a>
          </div>
        )}
      </div>
    </section>
  )
}
