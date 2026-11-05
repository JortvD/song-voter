import { useState } from 'react'

import { ApiError, api } from '../lib/api'

function formatNumber(value: number) {
  return value.toLocaleString()
}

export function AdminDangerZone({ onReset }: { onReset: () => void }) {
  const [confirm, setConfirm] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)

  async function reset() {
    if (confirm !== 'RESET' || busy) return

    const confirmed = window.confirm(
      'Delete every vote, matchup history and Elo rating for the active playlist? This cannot be undone.',
    )
    if (!confirmed) return

    setBusy(true)
    setError(null)
    setNotice(null)

    try {
      const result = await api.resetVotes(confirm)
      setNotice(
        `Reset complete — ${formatNumber(result.deleted_votes)} votes and ${formatNumber(result.deleted_matchups)} matchups removed.`,
      )
      setConfirm('')
      onReset()
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Reset failed')
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className="border border-accent/50 bg-accent/[0.03]">
      <div className="flex items-center justify-between border-b border-accent/30 px-5 py-3">
        <h2 className="font-mono text-[10px] uppercase tracking-[0.22em] text-accent">
          Danger zone
        </h2>
        <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-accent/70">
          Irreversible
        </span>
      </div>

      <div className="p-5">
        <p className="max-w-2xl text-sm leading-relaxed text-muted">
          Delete all votes, matchups and Elo ratings for the active playlist, and
          hide the shared chart. Users, invites and the imported playlist are
          kept.
        </p>

        <div className="mt-4 flex flex-col gap-2 sm:flex-row">
          <input
            type="text"
            value={confirm}
            onChange={(event) => setConfirm(event.target.value.toUpperCase())}
            placeholder="Type RESET"
            className="field max-w-48 font-mono text-xs tracking-[0.18em]"
            aria-label="Type RESET to confirm"
          />
          <button
            type="button"
            onClick={() => void reset()}
            disabled={confirm !== 'RESET' || busy}
            className="btn border-accent bg-accent hover:bg-accent/80"
          >
            {busy ? 'Resetting…' : 'Reset all votes'}
          </button>
        </div>

        {notice && (
          <p className="mt-3 border-l-2 border-ok bg-ok/5 px-3 py-2 text-xs text-ok">
            {notice}
          </p>
        )}
        {error && (
          <p className="mt-3 border-l-2 border-accent bg-accent/5 px-3 py-2 text-xs text-accent">
            {error}
          </p>
        )}
      </div>
    </section>
  )
}
