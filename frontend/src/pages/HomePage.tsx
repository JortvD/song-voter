import { useEffect } from 'react'
import { Link } from 'react-router-dom'

import { AdminInvites } from '../components/AdminInvites'
import { AdminPlaylist } from '../components/AdminPlaylist'
import { AdminReveal } from '../components/AdminReveal'
import { AuthCard } from '../components/AuthCard'
import { useAuth } from '../context/AuthContext'
import type { User } from '../lib/types'

function formatDate(value: string): string {
  return new Date(value).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}

function HowItWorks() {
  const steps = [
    {
      title: 'Claim an invite',
      body: 'One code, one account. No email, no fuss.',
    },
    {
      title: 'Pick between two',
      body: 'Press Space to play and switch songs, then 1 or 2 to pick a side.',
    },
    {
      title: 'Watch it settle',
      body: 'Your Ranking updates after every pick. The Standings stay sealed until The Reveal.',
    },
  ]

  return (
    <section className="panel">
      <div className="flex items-center justify-between border-b border-line px-5 py-3">
        <h2 className="label">The idea</h2>
        <span className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted">
          Three steps
        </span>
      </div>
      <ol>
        {steps.map((step, index) => (
          <li
            key={step.title}
            className="grid grid-cols-[2.5rem_1fr] gap-4 border-b border-line px-5 py-5 last:border-b-0"
          >
            <span className="font-serif text-3xl leading-none text-accent">
              {String(index + 1).padStart(2, '0')}
            </span>
            <div>
              <h3 className="text-sm font-semibold">{step.title}</h3>
              <p className="mt-1 text-sm leading-relaxed text-muted">{step.body}</p>
            </div>
          </li>
        ))}
      </ol>
    </section>
  )
}

function WelcomePanel({ user }: { user: User }) {
  return (
    <section className="panel">
      <div className="flex items-center justify-between border-b border-line px-5 py-3">
        <h2 className="label">Signed in</h2>
        <span className="font-mono text-[10px] uppercase tracking-[0.16em] text-muted">
          Picking is open
        </span>
      </div>
      <div className="p-5 sm:p-6">
        <h2 className="font-serif text-4xl leading-[1.05]">
          Good to see you,
          <br />
          {user.username}.
        </h2>
        <p className="mt-4 max-w-md text-sm leading-relaxed text-muted">
          Pick between two songs, over and over. Press Space to play and switch,
          pick with 1 and 2. Your Ranking updates after every pick — The
          Standings stay sealed until The Reveal.
        </p>
        <div className="mt-6 flex flex-wrap gap-3">
          <Link to="/duel" className="btn">
            Start picking
          </Link>
          <Link to="/your-ranking" className="btn-ghost">
            Your Ranking
          </Link>
          <Link to="/standings" className="btn-ghost">
            Standings
          </Link>
        </div>
      </div>
    </section>
  )
}

function AccountCard({ user }: { user: User }) {
  return (
    <section className="panel">
      <div className="flex items-center justify-between border-b border-line px-5 py-3">
        <h2 className="label">Account</h2>
        <span
          className={`border px-1.5 py-0.5 font-mono text-[9px] uppercase tracking-[0.14em] ${
            user.is_admin ? 'border-accent/40 text-accent' : 'border-line text-muted'
          }`}
        >
          {user.is_admin ? 'Admin' : 'Voter'}
        </span>
      </div>
      <div className="p-5">
        <p className="font-serif text-3xl leading-none">{user.username}</p>
        <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
          Member since {formatDate(user.created_at)}
        </p>
        <div className="mt-5 border-t border-line pt-4">
          <p className="label">Your Ranking</p>
          <p className="mt-1.5 text-sm leading-relaxed text-muted">
            Every pick builds your own score. The Standings stay sealed until The
            Reveal.
          </p>
          <div className="mt-4 flex flex-wrap gap-2">
            <Link to="/duel" className="btn">
              Duel
            </Link>
            <Link to="/your-ranking" className="btn-ghost">
              Open Ranking
            </Link>
            <Link to="/standings" className="btn-ghost">
              Standings
            </Link>
            {user.is_admin && (
              <Link to="/admin" className="btn-ghost">
                Admin dashboard
              </Link>
            )}
          </div>
        </div>
      </div>
    </section>
  )
}

export function HomePage() {
  const { user } = useAuth()

  useEffect(() => {
    if (window.location.hash !== '#auth' || user) return

    const timer = window.setTimeout(() => {
      document.getElementById('auth')?.scrollIntoView({ behavior: 'smooth', block: 'start' })
    }, 80)

    return () => window.clearTimeout(timer)
  }, [user])

  return (
    <>
      <section className="border-b border-line">
        <div className="mx-auto w-full max-w-6xl px-5 py-14 sm:px-8 sm:py-20">
          <div className="max-w-2xl">
            <p className="label flex items-center gap-2">
              <span className="size-1.5 bg-accent" />
              The people&apos;s playlist
            </p>
            <h1 className="mt-6 font-serif text-6xl leading-[0.92] tracking-[-0.02em] sm:text-7xl lg:text-8xl">
              Two songs.
              <br />
              One pick.<span className="text-accent">*</span>
            </h1>
            <p className="mt-6 max-w-md text-[15px] leading-relaxed text-muted">
              A playlist becomes a contest. Two tracks enter a duel, and every
              pick nudges the shared standings.
            </p>
            <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.18em] text-muted/70">
              * repeated a thousand times, by everyone.
            </p>

            <div className="mt-7 flex flex-wrap items-center gap-3">
              {user ? (
                <>
                  <Link to="/duel" className="btn">
                    Start picking
                  </Link>
                  <Link to="/your-ranking" className="btn-ghost">
                    Your Ranking
                  </Link>
                  <Link to="/standings" className="btn-ghost">
                    Standings
                  </Link>
                </>
              ) : (
                <>
                  <a href="#auth" className="btn">
                    Sign in to pick
                  </a>
                  <Link to="/standings" className="btn-ghost">
                    See the standings
                  </Link>
                </>
              )}
            </div>
          </div>
        </div>
      </section>

      <main className="mx-auto w-full max-w-6xl flex-1 px-5 py-10 sm:px-8 sm:py-14">
        <div className="grid items-start gap-6 lg:grid-cols-[1.35fr_0.65fr] lg:gap-8">
          <div className="space-y-6">
            {user?.is_admin ? (
              <>
                <AdminPlaylist />
                <AdminReveal />
                <HowItWorks />
              </>
            ) : user ? (
              <WelcomePanel user={user} />
            ) : (
              <HowItWorks />
            )}
          </div>

          <div className="space-y-6">
            {user ? <AccountCard user={user} /> : <AuthCard />}
            {user?.is_admin && <AdminInvites />}
          </div>
        </div>
      </main>
    </>
  )
}
