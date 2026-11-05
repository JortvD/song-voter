import { useEffect, useState } from 'react'

import { ApiError, api } from '../lib/api'
import type { Invite } from '../lib/types'

export function AdminInvites() {
  const [invites, setInvites] = useState<Invite[]>([])
  const [loading, setLoading] = useState(true)
  const [creating, setCreating] = useState(false)
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

  async function copyCode(code: string) {
    try {
      await navigator.clipboard.writeText(code)
      setCopied(code)
      window.setTimeout(() => setCopied(null), 1500)
    } catch {
      // Clipboard access can be denied; the code stays visible to copy by hand.
    }
  }

  return (
    <section className="panel">
      <div className="flex items-center justify-between border-b border-line px-5 py-3">
        <h2 className="label">Invites</h2>
        <button
          type="button"
          onClick={() => void createInvite()}
          disabled={creating}
          className="btn-ghost"
        >
          {creating ? 'Creating…' : 'Generate'}
        </button>
      </div>

      <div className="p-5">
        {error && (
          <p className="mb-4 border-l-2 border-accent bg-accent/5 px-3 py-2 text-xs text-accent">
            {error}
          </p>
        )}

        {loading ? (
          <div className="space-y-2">
            {[0, 1].map((index) => (
              <div key={index} className="h-10 animate-pulse border border-line bg-paper" />
            ))}
          </div>
        ) : invites.length === 0 ? (
          <p className="border border-dashed border-line px-3 py-6 text-center text-xs text-muted">
            No invite codes yet. Generate one for each voter.
          </p>
        ) : (
          <ul className="max-h-64 divide-y divide-line overflow-y-auto border-y border-line">
            {invites.map((invite) => (
              <li
                key={invite.id}
                className="flex items-center justify-between gap-3 py-2.5"
              >
                <div className="min-w-0">
                  <code className="font-mono text-sm tracking-[0.14em]">{invite.code}</code>
                  <p className="mt-0.5 truncate font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
                    {invite.used_by_username
                      ? `Used by ${invite.used_by_username}`
                      : 'Open'}
                  </p>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  <span
                    className={`border px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-[0.14em] ${
                      invite.used_by_username
                        ? 'border-line text-muted'
                        : 'border-ok/40 text-ok'
                    }`}
                  >
                    {invite.used_by_username ? 'Used' : 'Open'}
                  </span>
                  <button
                    type="button"
                    onClick={() => void copyCode(invite.code)}
                    className="border border-line px-2 py-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted transition hover:border-ink hover:text-ink"
                  >
                    {copied === invite.code ? 'Copied' : 'Copy'}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  )
}
