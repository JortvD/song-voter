import { useEffect, useState } from 'react'

import { ApiError, api } from '../lib/api'
import type { Invite } from '../lib/types'

function formatDate(value: string) {
  return new Date(value).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}

function inviteStatus(invite: Invite): 'open' | 'used' | 'expired' {
  if (invite.used_at) return 'used'
  if (invite.expires_at && new Date(invite.expires_at).getTime() <= Date.now()) {
    return 'expired'
  }
  return 'open'
}

function inviteLink(code: string): string {
  const url = new URL('/', window.location.origin)
  url.searchParams.set('invite', code)
  return url.toString()
}

export function AdminInvites() {
  const [invites, setInvites] = useState<Invite[]>([])
  const [loading, setLoading] = useState(true)
  const [creating, setCreating] = useState(false)
  const [revoking, setRevoking] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [copied, setCopied] = useState<string | null>(null)

  useEffect(() => {
    let active = true

    api
      .listInvites()
      .then((list) => {
        if (active) setInvites(list)
      })
      .catch((caught: unknown) => {
        if (active) {
          setError(caught instanceof ApiError ? caught.message : 'Could not load invites')
        }
      })
      .finally(() => {
        if (active) setLoading(false)
      })

    return () => {
      active = false
    }
  }, [])

  async function createInvite() {
    setCreating(true)
    setError(null)

    try {
      const invite = await api.createInvite()
      setInvites((current) => [invite, ...current])
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not create an invite')
    } finally {
      setCreating(false)
    }
  }

  async function copyText(value: string) {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(value)
      window.setTimeout(() => setCopied(null), 1500)
    } catch {
      // Clipboard access can be denied; the code stays visible to copy by hand.
    }
  }

  async function copyCode(code: string) {
    await copyText(code)
  }

  async function copyLink(code: string) {
    await copyText(inviteLink(code))
  }

  async function revoke(invite: Invite) {
    if (!window.confirm(`Revoke invite ${invite.code}?`)) return

    setRevoking(invite.id)
    setError(null)

    try {
      await api.revokeInvite(invite.id)
      setInvites((current) => current.filter((item) => item.id !== invite.id))
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not revoke the invite')
    } finally {
      setRevoking(null)
    }
  }

  const open = invites.filter((invite) => inviteStatus(invite) === 'open').length
  const used = invites.filter((invite) => inviteStatus(invite) === 'used').length
  const expired = invites.filter((invite) => inviteStatus(invite) === 'expired').length

  return (
    <section className="panel">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-line px-5 py-3">
        <div>
          <h2 className="label">Invites</h2>
          <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {open} open · {used} used · {expired} expired
          </p>
        </div>
        <button
          type="button"
          onClick={() => void createInvite()}
          disabled={creating}
          className="btn-ghost"
        >
          {creating ? 'Creating…' : 'Generate code'}
        </button>
      </div>

      <div className="p-5">
        {error && (
          <p className="mb-4 border-l-2 border-danger bg-danger/5 px-3 py-2 text-xs text-danger">
            {error}
          </p>
        )}

        {loading ? (
          <div className="space-y-2">
            {[0, 1, 2].map((index) => (
              <div key={index} className="h-12 animate-pulse border border-line bg-paper" />
            ))}
          </div>
        ) : invites.length === 0 ? (
          <p className="border border-dashed border-line px-3 py-6 text-center text-xs text-muted">
            No invite codes yet. Generate one for each voter.
          </p>
        ) : (
          <ul className="max-h-96 divide-y divide-line overflow-y-auto border-y border-line">
            {invites.map((invite) => {
              const status = inviteStatus(invite)
              const link = inviteLink(invite.code)
              const statusClass =
                status === 'open'
                  ? 'border-ok/40 text-ok'
                  : status === 'expired'
                    ? 'border-danger/40 text-danger'
                    : 'border-line text-muted'

              return (
                <li
                  key={invite.id}
                  className="flex flex-wrap items-center justify-between gap-3 py-3"
                >
                  <div className="min-w-0">
                    <code className="font-mono text-sm tracking-[0.14em]">
                      {invite.code}
                    </code>
                    <p className="mt-0.5 truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                      {invite.used_by_username
                        ? `Used by ${invite.used_by_username}`
                        : status === 'expired'
                          ? 'Expired'
                          : `Created ${formatDate(invite.created_at)}`}
                      {invite.expires_at && status === 'open'
                        ? ` · expires ${formatDate(invite.expires_at)}`
                        : ''}
                    </p>
                  </div>

                  <div className="flex shrink-0 flex-wrap items-center gap-2">
                    <span
                      className={`border px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-[0.12em] ${statusClass}`}
                    >
                      {status}
                    </span>
                    <button
                      type="button"
                      onClick={() => void copyCode(invite.code)}
                      className="border border-line px-2 py-1 font-mono text-[9px] uppercase tracking-[0.12em] text-muted transition hover:border-ink hover:text-ink"
                    >
                      {copied === invite.code ? 'Copied' : 'Copy code'}
                    </button>
                    {status === 'open' && (
                      <>
                        <button
                          type="button"
                          onClick={() => void copyLink(invite.code)}
                          className="border border-line px-2 py-1 font-mono text-[9px] uppercase tracking-[0.12em] text-muted transition hover:border-ink hover:text-ink"
                        >
                          {copied === link ? 'Copied' : 'Copy link'}
                        </button>
                        <button
                          type="button"
                          onClick={() => void revoke(invite)}
                          disabled={revoking === invite.id}
                          className="border border-danger/40 px-2 py-1 font-mono text-[9px] uppercase tracking-[0.12em] text-danger transition hover:border-danger disabled:opacity-40"
                        >
                          {revoking === invite.id ? 'Revoking…' : 'Revoke'}
                        </button>
                      </>
                    )}
                  </div>
                </li>
              )
            })}
          </ul>
        )}
      </div>
    </section>
  )
}
