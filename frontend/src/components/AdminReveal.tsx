import { useEffect, useState } from 'react'

import { ApiError, api } from '../lib/api'
import type { ChartStatus } from '../lib/types'

function formatNumber(value: number) {
  return value.toLocaleString()
}

export function AdminReveal() {
  const [status, setStatus] = useState<ChartStatus | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true

    api
      .getChartStatus()
      .then((next) => {
        if (active) setStatus(next)
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

  async function toggleReveal() {
    if (!status) return

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
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not change The Reveal')
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
          {status?.revealed ? 'Live' : 'Sealed'}
        </span>
      </div>

      <div className="p-5">
        <p className="text-sm leading-relaxed text-muted">
          The shared standings stay hidden from voters until The Reveal.
          Picking continues afterwards, and you can seal them again.
        </p>

        {status && (
          <p className="mt-3 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {formatNumber(status.total_tracks)} songs ·{' '}
            {formatNumber(status.votes_cast)} picks
          </p>
        )}

        {error && (
          <p className="mt-3 border-l-2 border-danger bg-danger/5 px-3 py-2 text-xs text-danger">
            {error}
          </p>
        )}

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
      </div>
    </section>
  )
}
