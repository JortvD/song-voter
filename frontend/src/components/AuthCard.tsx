import { useState } from 'react'
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
    <section id="auth" className="panel">
      <div className="flex items-center justify-between border-b border-line px-5 py-3">
        <h2 className="label">Membership</h2>
        <span className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted">
          Invite only
        </span>
      </div>

      <div className="p-5">
        <div className="flex gap-5 border-b border-line">
          {(['login', 'register'] as const).map((value) => (
            <button
              key={value}
              type="button"
              onClick={() => switchMode(value)}
              className={`-mb-px border-b-2 pb-2.5 font-mono text-[11px] uppercase tracking-[0.16em] transition ${
                mode === value
                  ? 'border-ink text-ink'
                  : 'border-transparent text-muted hover:text-ink'
              }`}
            >
              {value === 'login' ? 'Sign in' : 'Register'}
            </button>
          ))}
        </div>

        <form onSubmit={handleSubmit} className="mt-5 space-y-4">
          <div>
            <label htmlFor="username" className="label mb-1.5 block">
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
              className="field"
            />
          </div>

          <div>
            <label htmlFor="password" className="label mb-1.5 block">
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
              className="field"
            />
          </div>

          {mode === 'register' && (
            <div>
              <label htmlFor="invite-code" className="label mb-1.5 block">
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
                className="field font-mono tracking-[0.18em]"
              />
            </div>
          )}

          {error && (
            <p role="alert" className="border-l-2 border-accent bg-accent/5 px-3 py-2 text-sm text-accent">
              {error}
            </p>
          )}

          <button type="submit" disabled={busy} className="btn w-full">
            {busy ? 'Working…' : mode === 'login' ? 'Sign in' : 'Create account'}
          </button>
        </form>
      </div>

      <p className="border-t border-line px-5 py-3 text-xs leading-relaxed text-muted">
        Registration is invite-only. Ask the admin for a code, or use the admin
        account configured on the server.
      </p>
    </section>
  )
}
