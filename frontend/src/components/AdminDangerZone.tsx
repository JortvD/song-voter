import { useState } from 'react'

import { ApiError, api } from '../lib/api'

function formatNumber(value: number) {
  return value.toLocaleString()
}

type Props = {
  onReset: () => void
  onRemove: () => void
}

export function AdminDangerZone({ onReset, onRemove }: Props) {
  const [resetConfirm, setResetConfirm] = useState('')
  const [removeConfirm, setRemoveConfirm] = useState('')
  const [busy, setBusy] = useState<'reset' | 'remove' | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)

  async function resetVotes() {
    if (resetConfirm !== 'RESET' || busy) return

    const confirmed = window.confirm(
      'Delete every pick, duel history and score for the active playlist? This cannot be undone.',
    )
    if (!confirmed) return

    setBusy('reset')
    setError(null)
    setNotice(null)

    try {
      const result = await api.resetVotes(resetConfirm)
      setNotice(
        `Reset complete — ${formatNumber(result.deleted_votes)} picks and ${formatNumber(result.deleted_matchups)} duels removed.`,
      )
      setResetConfirm('')
      onReset()
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Reset failed')
    } finally {
      setBusy(null)
    }
  }

  async function removePlaylist() {
    if (removeConfirm !== 'REMOVE' || busy) return

    const confirmed = window.confirm(
      'Remove the current playlist, including all its tracks, picks, duels and scores? Voters, invites and the Spotify connection are kept.',
    )
    if (!confirmed) return

    setBusy('remove')
    setError(null)
    setNotice(null)

    try {
      const result = await api.removePlaylist(removeConfirm)
      setNotice(
        `Playlist removed — ${formatNumber(result.removed_tracks)} tracks and ${formatNumber(result.removed_votes)} picks deleted.`,
      )
      setRemoveConfirm('')
      onRemove()
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not remove the playlist')
    } finally {
      setBusy(null)
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

      <div className="grid gap-4 p-5 sm:grid-cols-2">
        <div className="border border-line bg-card p-4">
          <p className="label">Picks &amp; rankings</p>
          <p className="mt-2 text-sm leading-relaxed text-muted">
            Delete all picks, duels and scores for the active playlist, and seal
            the standings. Voters and invites are kept.
          </p>
          <div className="mt-4 flex flex-col gap-2 sm:flex-row">
            <input
              type="text"
              value={resetConfirm}
              onChange={(event) => setResetConfirm(event.target.value.toUpperCase())}
              placeholder="Type RESET"
              className="field max-w-40 font-mono text-xs tracking-[0.18em]"
              aria-label="Type RESET to confirm resetting picks"
            />
            <button
              type="button"
              onClick={() => void resetVotes()}
              disabled={resetConfirm !== 'RESET' || busy !== null}
              className="btn border-accent bg-accent hover:bg-accent/80"
            >
              {busy === 'reset' ? 'Resetting…' : 'Reset all picks'}
            </button>
          </div>
        </div>

        <div className="border border-line bg-card p-4">
          <p className="label">Playlist</p>
          <p className="mt-2 text-sm leading-relaxed text-muted">
            Remove the current playlist and return to the empty state. Its
            tracks, picks, duels and scores are deleted. Voters, invites and
            the Spotify connection are kept.
          </p>
          <div className="mt-4 flex flex-col gap-2 sm:flex-row">
            <input
              type="text"
              value={removeConfirm}
              onChange={(event) => setRemoveConfirm(event.target.value.toUpperCase())}
              placeholder="Type REMOVE"
              className="field max-w-40 font-mono text-xs tracking-[0.18em]"
              aria-label="Type REMOVE to confirm removing the playlist"
            />
            <button
              type="button"
              onClick={() => void removePlaylist()}
              disabled={removeConfirm !== 'REMOVE' || busy !== null}
              className="btn border-accent bg-accent hover:bg-accent/80"
            >
              {busy === 'remove' ? 'Removing…' : 'Remove playlist'}
            </button>
          </div>
        </div>
      </div>

      {(notice || error) && (
        <div className="px-5 pb-5">
          {notice && (
            <p className="border-l-2 border-ok bg-ok/5 px-3 py-2 text-xs text-ok">
              {notice}
            </p>
          )}
          {error && (
            <p className="mt-3 border-l-2 border-accent bg-accent/5 px-3 py-2 text-xs text-accent">
              {error}
            </p>
          )}
        </div>
      )}
    </section>
  )
}
