import type { ReactNode } from 'react'

import { useAuth } from '../context/AuthContext'

function AdminGate({
  title,
  body,
  action,
}: {
  title: string
  body: string
  action: ReactNode
}) {
  return (
    <section className="pv-card mx-auto mt-8 w-full max-w-xl overflow-hidden">
      <div className="flex items-center justify-between gap-3 border-b-2 border-ink bg-ink px-5 py-3 text-paper">
        <span className="pv-sticker pv-sticker-butter px-1.5 py-0.5 text-[9px]">
          Admin
        </span>
        <span className="font-serif text-lg leading-none tracking-[-0.02em]">
          Mission control
        </span>
      </div>
      <div className="p-6">
        <h1 className="font-serif text-3xl leading-tight tracking-[-0.03em]">{title}</h1>
        <p className="mt-3 text-sm leading-relaxed text-muted">{body}</p>
        {action}
      </div>
    </section>
  )
}

export function RequireAdmin({ children }: { children: ReactNode }) {
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
      <AdminGate
        title="Sign in first."
        body="The dashboard is only available to the admin account."
        action={
          <a href="/#auth" className="btn mt-5 inline-flex">
            Go to sign in
          </a>
        }
      />
    )
  }

  if (!user.is_admin) {
    return (
      <AdminGate
        title="Admins only."
        body="This account doesn't have admin access."
        action={
          <a href="/" className="btn mt-5 inline-flex">
            Back home
          </a>
        }
      />
    )
  }

  return <>{children}</>
}
