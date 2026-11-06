import { useState } from 'react'

import { useAuth } from '../context/AuthContext'
import { ApiError, api } from '../lib/api'
import type { AdminUserRow } from '../lib/types'

type Props = {
  user: AdminUserRow
  isSelf: boolean
  onChanged: (message: string) => void
  onDeleted: (message: string) => void
}

function formatNumber(value: number) {
  return value.toLocaleString()
}

function formatDateTime(value: string | null) {
  if (!value) return 'Never'
  return new Date(value).toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

export function AdminUserActions({ user, isSelf, onChanged, onDeleted }: Props) {
  const { refreshUser } = useAuth()
  const [renameValue, setRenameValue] = useState(user.username)
  const [passwordValue, setPasswordValue] = useState('')
  const [clearConfirm, setClearConfirm] = useState('')
  const [deleteConfirm, setDeleteConfirm] = useState('')
  const [busy, setBusy] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  async function renameUser() {
    const username = renameValue.trim()
    if (!username || username === user.username || busy) return

    setBusy('rename')
    setError(null)

    try {
      const result = await api.renameAdminUser(user.id, username)
      setRenameValue(result.username)
      if (isSelf) await refreshUser()
      onChanged(`Renamed ${user.username} to ${result.username}.`)
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not rename this voter')
    } finally {
      setBusy(null)
    }
  }

  async function resetPassword() {
    if (passwordValue.length < 8 || busy) return

    setBusy('password')
    setError(null)

    try {
      const result = await api.resetAdminUserPassword(user.id, passwordValue)
      setPasswordValue('')
      onChanged(
        `Password reset for ${result.username}. All of their existing sessions were signed out.`,
      )
      if (isSelf) await refreshUser()
    } catch (caught) {
      setError(
        caught instanceof ApiError ? caught.message : 'Could not reset this password',
      )
    } finally {
      setBusy(null)
    }
  }

  async function toggleDisabled() {
    if (busy) return

    const disabled = !user.disabled_at
    if (
      disabled &&
      !window.confirm(
        `Disable ${user.username}? They will be signed out immediately and unable to sign in until re-enabled.`,
      )
    ) {
      return
    }

    setBusy('disabled')
    setError(null)

    try {
      const result = await api.setAdminUserDisabled(user.id, disabled)
      onChanged(
        result.disabled
          ? `${result.username} has been disabled and signed out.`
          : `${result.username} can sign in again.`,
      )
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : 'Could not change this account status',
      )
    } finally {
      setBusy(null)
    }
  }

  async function forceLogout() {
    if (busy) return
    if (!window.confirm(`Sign ${user.username} out of every session?`)) return

    setBusy('logout')
    setError(null)

    try {
      await api.logoutAdminUser(user.id)
      onChanged(`${user.username} has been signed out everywhere.`)
      if (isSelf) await refreshUser()
    } catch (caught) {
      setError(
        caught instanceof ApiError ? caught.message : 'Could not revoke this session',
      )
    } finally {
      setBusy(null)
    }
  }

  async function clearVotes() {
    if (clearConfirm !== 'CLEAR' || busy) return
    if (
      !window.confirm(
        `Delete all ${formatNumber(user.votes_cast)} picks and ${formatNumber(user.matchups_seen)} duels from ${user.username} in the active playlist? Their coverage starts over and the standings will be rebuilt without those picks.`,
      )
    ) {
      return
    }

    setBusy('clear')
    setError(null)

    try {
      const result = await api.clearAdminUserVotes(user.id, clearConfirm)
      setClearConfirm('')
      onChanged(
        `Cleared ${formatNumber(result.deleted_votes)} picks and ` +
          `${formatNumber(result.deleted_duels)} duels for ${result.username}. ` +
          `Rebuilt ${formatNumber(result.tracks_updated)} tracks from ${formatNumber(result.votes_replayed)} remaining picks.`,
      )
    } catch (caught) {
      setError(
        caught instanceof ApiError ? caught.message : 'Could not clear these picks',
      )
    } finally {
      setBusy(null)
    }
  }

  async function deleteUser() {
    if (deleteConfirm !== user.username || busy) return
    if (
      !window.confirm(
        `Permanently delete ${user.username} and all of their picks and duel history? The standings will be rebuilt. This cannot be undone.`,
      )
    ) {
      return
    }

    setBusy('delete')
    setError(null)

    try {
      const result = await api.deleteAdminUser(user.id, deleteConfirm)
      onDeleted(
        `Deleted ${result.username} — ${formatNumber(result.deleted_votes)} picks and ` +
          `${formatNumber(result.deleted_duels)} duels removed; ` +
          `${result.rebuilt_playlists} playlist${result.rebuilt_playlists === 1 ? '' : 's'} rebuilt.`,
      )
    } catch (caught) {
      setError(
        caught instanceof ApiError ? caught.message : 'Could not delete this voter',
      )
    } finally {
      setBusy(null)
    }
  }

  return (
    <div className="space-y-4 border-t-2 border-line bg-paper/70 px-4 py-4">
      <div className="flex flex-wrap gap-x-6 gap-y-1 font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
        <span>Joined {new Date(user.created_at).toLocaleDateString()}</span>
        <span>Last pick {formatDateTime(user.last_vote_at)}</span>
        <span>Last duel {formatDateTime(user.last_seen_at)}</span>
      </div>

      {error && (
        <p className="border-l-2 border-danger bg-danger/5 px-3 py-2 text-xs text-danger">
          {error}
        </p>
      )}

      <div className="grid gap-3 md:grid-cols-2">
        <section className="border border-line bg-card p-3">
          <p className="label">Rename voter</p>
          <p className="mt-1 text-xs leading-relaxed text-muted">
            Changing the name does not sign them out.
          </p>
          <div className="mt-3 flex gap-2">
            <input
              type="text"
              value={renameValue}
              onChange={(event) => setRenameValue(event.target.value)}
              className="pv-admin-input"
              aria-label={`Username for ${user.username}`}
            />
            <button
              type="button"
              onClick={() => void renameUser()}
              disabled={
                busy !== null ||
                !renameValue.trim() ||
                renameValue.trim() === user.username
              }
              className="pv-admin-button shrink-0"
            >
              {busy === 'rename' ? 'Saving…' : 'Save'}
            </button>
          </div>
        </section>

        <section className="border border-line bg-card p-3">
          <p className="label">Access &amp; sessions</p>
          <p className="mt-1 text-xs leading-relaxed text-muted">
            {isSelf
              ? 'You cannot disable your own account.'
              : user.disabled_at
                ? 'This account is disabled and cannot sign in.'
                : 'Active account.'}
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => void toggleDisabled()}
              disabled={busy !== null || isSelf}
              className={`pv-admin-button ${user.disabled_at ? '' : 'pv-admin-button-danger'}`}
            >
              {busy === 'disabled'
                ? 'Saving…'
                : user.disabled_at
                  ? 'Enable account'
                  : 'Disable account'}
            </button>
            <button
              type="button"
              onClick={() => void forceLogout()}
              disabled={busy !== null}
              className="pv-admin-button"
            >
              {busy === 'logout' ? 'Signing out…' : 'Sign out everywhere'}
            </button>
          </div>

          <div className="mt-3 flex gap-2">
            <input
              type="password"
              value={passwordValue}
              onChange={(event) => setPasswordValue(event.target.value)}
              placeholder="New password (8+ characters)"
              className="pv-admin-input"
              autoComplete="new-password"
              aria-label={`New password for ${user.username}`}
            />
            <button
              type="button"
              onClick={() => void resetPassword()}
              disabled={busy !== null || passwordValue.length < 8}
              className="pv-admin-button shrink-0"
            >
              {busy === 'password' ? 'Resetting…' : 'Reset'}
            </button>
          </div>
          <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.1em] text-muted">
            Reset also signs out every session.
          </p>
        </section>

        <section className="border border-line bg-card p-3">
          <p className="label">Picks</p>
          <p className="mt-1 text-xs leading-relaxed text-muted">
            Delete their {formatNumber(user.votes_cast)} picks and{' '}
            {formatNumber(user.matchups_seen)} duels in the active playlist, then
            rebuild the standings and vote ledger. Their coverage starts over.
          </p>
          <div className="mt-3 flex gap-2">
            <input
              type="text"
              value={clearConfirm}
              onChange={(event) =>
                setClearConfirm(event.target.value.toUpperCase())
              }
              placeholder="Type CLEAR"
              className="pv-admin-input font-mono text-xs tracking-[0.16em]"
              aria-label={`Type CLEAR to remove picks for ${user.username}`}
            />
            <button
              type="button"
              onClick={() => void clearVotes()}
              disabled={busy !== null || clearConfirm !== 'CLEAR'}
              className="pv-admin-button shrink-0"
            >
              {busy === 'clear' ? 'Clearing…' : 'Clear picks'}
            </button>
          </div>
        </section>

        <section className="border border-danger/40 bg-danger/[0.03] p-3">
          <p className="label text-danger">Delete voter</p>
          <p className="mt-1 text-xs leading-relaxed text-muted">
            This removes the account, every pick and every duel across all playlists.
            Type <span className="font-mono text-ink">{user.username}</span> to confirm.
          </p>
          {isSelf ? (
            <p className="mt-3 font-mono text-[9px] uppercase tracking-[0.12em] text-danger">
              You cannot delete your own account.
            </p>
          ) : (
            <div className="mt-3 flex gap-2">
              <input
                type="text"
                value={deleteConfirm}
                onChange={(event) => setDeleteConfirm(event.target.value)}
                placeholder={user.username}
                className="pv-admin-input font-mono text-xs"
                aria-label={`Type ${user.username} to delete this voter`}
              />
              <button
                type="button"
                onClick={() => void deleteUser()}
                disabled={busy !== null || deleteConfirm !== user.username}
                className="pv-admin-button pv-admin-button-danger shrink-0"
              >
                {busy === 'delete' ? 'Deleting…' : 'Delete'}
              </button>
            </div>
          )}
        </section>
      </div>
    </div>
  )
}
