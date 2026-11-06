import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'

import { useAuth } from '../context/AuthContext'
import { ApiError } from '../lib/api'

type Mode = 'login' | 'register'

export function AuthCard() {
  const { login, register } = useAuth()
  const [mode, setMode] = useState<Mode>('login')
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [inviteCode, setInviteCode] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    const invite = new URLSearchParams(window.location.search)
      .get('invite')
      ?.trim()
      .toUpperCase()

    if (!invite) return

    setMode('register')
    setInviteCode(invite)

    const timer = window.setTimeout(() => {
      document
        .getElementById('auth')
        ?.scrollIntoView({ behavior: 'smooth', block: 'start' })
    }, 80)

    return () => window.clearTimeout(timer)
  }, [])

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setError(null)
    setBusy(true)

    try {
      if (mode === 'login') {
        await login(username, password)
      } else {
        await register(username, password, inviteCode)
      }

      const url = new URL(window.location.href)
      if (url.searchParams.has('invite')) {
        url.searchParams.delete('invite')
        window.history.replaceState(
          {},
          '',
          `${url.pathname}${url.search}${url.hash}`,
        )
      }

      setPassword('')
      setInviteCode('')
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Something went wrong')
    } finally {
      setBusy(false)
    }
  }

  function switchMode(next: Mode) {
    setMode(next)
    setError(null)
  }

  return (
    <section id="auth" className="pv-card min-w-0 overflow-hidden">
      <div className="flex min-w-0 items-center justify-between gap-3 border-b-2 border-ink bg-ink px-5 py-3 text-paper">
        <div className="flex min-w-0 items-center gap-2">
          <span className="pv-sticker pv-sticker-butter px-1.5 py-0.5 text-[9px]">
            Membership
          </span>
          <span className="min-w-0 truncate font-serif text-lg leading-none tracking-[-0.02em]">
            Backstage pass
          </span>
        </div>
        <span className="hidden shrink-0 font-mono text-[9px] uppercase tracking-[0.16em] text-paper/85 sm:inline">
          Invite only
        </span>
      </div>

      <div className="p-5">
        <div className="flex gap-1 rounded-full border-2 border-ink bg-paper p-1">
          {(['login', 'register'] as const).map((value) => (
            <button
              key={value}
              type="button"
              onClick={() => switchMode(value)}
              className={`flex-1 rounded-full px-3 py-2 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] transition ${
                mode === value
                  ? 'bg-ink text-paper'
                  : 'text-muted hover:text-ink'
              }`}
            >
              {value === 'login' ? 'Sign in' : 'Register'}
            </button>
          ))}
        </div>

        <form onSubmit={handleSubmit} className="mt-5 space-y-4">
          <div>
            <label htmlFor="username" className="pv-label mb-1.5 block">
              Username
            </label>
            <input
              id="username"
              name="username"
              type="text"
              required
              autoComplete="username"
              value={username}
              onChange={(event) => setUsername(event.target.value)}
              placeholder="your-name"
              className="pv-input"
            />
          </div>

          <div>
            <label htmlFor="password" className="pv-label mb-1.5 block">
              Password
            </label>
            <input
              id="password"
              name="password"
              type="password"
              required
              autoComplete={mode === 'login' ? 'current-password' : 'new-password'}
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              placeholder="••••••••"
              className="pv-input"
            />
          </div>

          {mode === 'register' && (
            <div>
              <label htmlFor="invite-code" className="pv-label mb-1.5 block">
                Invite code
              </label>
              <input
                id="invite-code"
                name="invite-code"
                type="text"
                required
                autoComplete="off"
                value={inviteCode}
                onChange={(event) => setInviteCode(event.target.value.toUpperCase())}
                placeholder="A1B2C3D4E5"
                className="pv-input font-mono tracking-[0.18em]"
              />
            </div>
          )}

          {error && (
            <p
              role="alert"
              className="rounded-xl border-2 border-danger bg-danger/5 px-3 py-2 text-sm text-danger"
            >
              {error}
            </p>
          )}

          <button type="submit" disabled={busy} className="btn w-full">
            {busy ? 'Working…' : mode === 'login' ? 'Sign in' : 'Join the club'}
          </button>
        </form>
      </div>

      <p className="border-t-2 border-ink/15 px-5 py-3 text-xs leading-relaxed text-muted">
        Registration is invite-only. Ask the admin for a code, or use the admin
        account configured on the server.
      </p>
    </section>
  )
}
