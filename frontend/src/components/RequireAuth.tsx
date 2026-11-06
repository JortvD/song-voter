import type { ReactNode } from 'react'

import { useAuth } from '../context/AuthContext'

export function RequireAuth({ children }: { children: ReactNode }) {
  const { user, loading } = useAuth()

  if (loading) {
    return (
      <div className="mx-auto mt-10 w-full max-w-xl px-4">
        <p className="pv-label">Checking session…</p>
      </div>
    )
  }

  if (!user) {
    return (
      <section className="pv-card mx-auto mt-8 w-full max-w-xl overflow-hidden">
        <div className="flex items-center justify-between gap-3 border-b-2 border-ink bg-ink px-5 py-3 text-paper">
          <span className="pv-sticker pv-sticker-butter px-1.5 py-0.5 text-[9px]">
            Membership
          </span>
          <span className="font-serif text-lg leading-none tracking-[-0.02em]">
            Backstage pass
          </span>
        </div>
        <div className="p-6">
          <p className="pv-label">Membership required</p>
          <h1 className="mt-2 font-serif text-3xl leading-tight tracking-[-0.03em]">
            Sign in to continue.
          </h1>
          <p className="mt-3 text-sm leading-relaxed text-muted">
            Picking is invite-only. Ask the admin for a code, then sign in from
            the home page.
          </p>
          <a href="/#auth" className="btn mt-5 inline-flex">
            Go to sign in
          </a>
        </div>
      </section>
    )
  }

  return <>{children}</>
}
