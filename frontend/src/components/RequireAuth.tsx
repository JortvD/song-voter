import type { ReactNode } from 'react'

import { useAuth } from '../context/AuthContext'

export function RequireAuth({ children }: { children: ReactNode }) {
  const { user, loading } = useAuth()

  if (loading) {
    return <p className="label">Checking session…</p>
  }

  if (!user) {
    return (
      <section className="panel mx-auto max-w-xl p-6">
        <p className="label">Membership required</p>
        <h1 className="mt-3 font-serif text-3xl leading-tight">Sign in to continue.</h1>
        <p className="mt-3 text-sm leading-relaxed text-muted">
          Picking is invite-only. Ask the admin for a code, then sign in from
          the home page.
        </p>
        <a href="/#auth" className="btn mt-5 inline-flex">
          Go to sign in
        </a>
      </section>
    )
  }

  return <>{children}</>
}
