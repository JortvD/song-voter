import { useState } from 'react'

import { usePlaylist } from '../context/PlaylistContext'
import { ApiError, api } from '../lib/api'

function formatNumber(value: number) {
  return value.toLocaleString()
}

type Props = {
  onReset: () => void
  onRemove: () => void
  onOpenPlaylist?: () => void
}

export function AdminDangerZone({ onReset, onRemove, onOpenPlaylist }: Props) {
  const { status } = usePlaylist()
  const hasTape = Boolean(status?.playlist_id)
  const [resetConfirm, setResetConfirm] = useState('')
  const [removeConfirm, setRemoveConfirm] = useState('')
  const [fullConfirm, setFullConfirm] = useState('')
  const [busy, setBusy] = useState<'reset' | 'remove' | 'full' | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)

  async function resetVotes() {
    if (!hasTape || resetConfirm !== 'RESET' || busy) return

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
    if (!hasTape || removeConfirm !== 'REMOVE' || busy) return

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

  async function fullReset() {
    if (fullConfirm !== 'RESET EVERYTHING' || busy) return

    const confirmed = window.confirm(
      'Delete every playlist, track, vote, duel, invite and other user? The Spotify connection and your admin account survive. This cannot be undone.',
    )
    if (!confirmed) return

    setBusy('full')
    setError(null)
    setNotice(null)

    try {
      const result = await api.fullReset(fullConfirm)
      setNotice(
        `Full reset complete — ${formatNumber(result.deleted_users)} users, ${formatNumber(result.deleted_playlists)} playlists, ${formatNumber(result.deleted_tracks)} tracks and ${formatNumber(result.deleted_votes)} picks deleted. Spotify stays connected.`,
      )
      setFullConfirm('')
      onRemove()
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Full reset failed')
    } finally {
      setBusy(null)
    }
  }

  return (
    <section className="border border-accent/50 bg-accent/[0.03]">
      <div className="flex items-center justify-between border-b border-accent/30 px-5 py-3">
        <h2 className="font-mono text-[10px] uppercase tracking-[0.22em] text-danger">
          Danger zone
        </h2>
        <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-danger">
          Irreversible
        </span>
      </div>

      {status && !hasTape && (
        <div className="border-b border-line px-5 py-4">
          <div className="border border-dashed border-line bg-card px-4 py-3">
            <p className="text-sm font-medium text-ink">No tape loaded.</p>
            <p className="mt-1 text-sm leading-relaxed text-muted">
              Import a playlist before resetting picks or removing a tape. Full reset
              remains available because it also clears users and invites.
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
        </div>
      )}

      <div className="grid gap-4 p-5 sm:grid-cols-2 xl:grid-cols-3">
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
              disabled={!hasTape || busy !== null}
              className="field max-w-40 font-mono text-xs tracking-[0.18em] disabled:cursor-not-allowed disabled:opacity-50"
              aria-label="Type RESET to confirm resetting picks"
            />
            <button
              type="button"
              onClick={() => void resetVotes()}
              disabled={!hasTape || resetConfirm !== 'RESET' || busy !== null}
              className="btn border-danger bg-danger hover:bg-danger/90"
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
              disabled={!hasTape || busy !== null}
              className="field max-w-40 font-mono text-xs tracking-[0.18em] disabled:cursor-not-allowed disabled:opacity-50"
              aria-label="Type REMOVE to confirm removing the playlist"
            />
            <button
              type="button"
              onClick={() => void removePlaylist()}
              disabled={!hasTape || removeConfirm !== 'REMOVE' || busy !== null}
              className="btn border-danger bg-danger hover:bg-danger/90"
            >
              {busy === 'remove' ? 'Removing…' : 'Remove playlist'}
            </button>
          </div>
        </div>

        <div className="border border-danger/40 bg-danger/[0.03] p-4">
          <p className="label text-danger">Full reset</p>
          <p className="mt-2 text-sm leading-relaxed text-muted">
            Delete every playlist, track, vote, duel, invite and other user. The
            Spotify connection stays intact. Only your admin account survives, and
            you stay logged in. There is no archive.
          </p>
          <div className="mt-4 flex flex-col gap-2 sm:flex-row">
            <input
              type="text"
              value={fullConfirm}
              onChange={(event) => setFullConfirm(event.target.value.toUpperCase())}
              placeholder="Type RESET EVERYTHING"
              className="field max-w-56 font-mono text-xs tracking-[0.18em]"
              aria-label="Type RESET EVERYTHING to confirm the full reset"
            />
            <button
              type="button"
              onClick={() => void fullReset()}
              disabled={fullConfirm !== 'RESET EVERYTHING' || busy !== null}
              className="btn border-danger bg-danger hover:bg-danger/90"
            >
              {busy === 'full' ? 'Resetting…' : 'Full reset'}
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
            <p className="mt-3 border-l-2 border-danger bg-danger/5 px-3 py-2 text-xs text-danger">
              {error}
            </p>
          )}
        </div>
      )}
    </section>
  )
}
