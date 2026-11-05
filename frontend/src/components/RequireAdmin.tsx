import type { ReactNode } from 'react'

import { useAuth } from '../context/AuthContext'

export function RequireAdmin({ children }: { children: ReactNode }) {
  const { user, loading } = useAuth()

  if (loading) {
    return <p className="label">Checking session…</p>
  }

  if (!user) {
    return (
      <section className="panel mx-auto max-w-xl p-6">
        <p className="label">Admin</p>
        <h1 className="mt-3 font-serif text-3xl leading-tight">Sign in first.</h1>
        <p className="mt-3 text-sm leading-relaxed text-muted">
          The dashboard is only available to the admin account.
        </p>
        <a href="/#auth" className="btn mt-5 inline-flex">
          Go to sign in
        </a>
      </section>
    )
  }

  if (!user.is_admin) {
    return (
      <section className="panel mx-auto max-w-xl p-6">
        <p className="label">Admin</p>
        <h1 className="mt-3 font-serif text-3xl leading-tight">Not your room.</h1>
        <p className="mt-3 text-sm leading-relaxed text-muted">
          This account does not have admin access.
        </p>
        <a href="/" className="btn mt-5 inline-flex">
          Back home
        </a>
      </section>
    )
  }

  return <>{children}</>
}
