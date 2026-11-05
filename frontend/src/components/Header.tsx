import { Link, NavLink, useNavigate } from 'react-router-dom'

import { useAuth } from '../context/AuthContext'

const navClass = ({ isActive }: { isActive: boolean }) =>
  `font-mono text-[10px] uppercase tracking-[0.16em] transition ${
    isActive
      ? 'text-ink underline decoration-accent decoration-2 underline-offset-8'
      : 'text-muted hover:text-ink'
  }`

export function Header() {
  const { user, loading, logout } = useAuth()
  const navigate = useNavigate()

  async function handleLogout() {
    await logout()
    navigate('/')
  }

  return (
    <header className="border-b border-ink/15">
      <div className="mx-auto flex w-full max-w-6xl items-center justify-between gap-6 px-5 py-4 sm:px-8">
        <div className="flex items-center gap-6">
          <Link to="/" className="flex items-center gap-3">
            <span className="grid size-9 place-items-center bg-ink font-serif text-lg leading-none text-paper">
              U
            </span>
            <span className="font-mono text-[10px] font-medium uppercase leading-relaxed tracking-[0.26em]">
              ULL / Top 2000
            </span>
          </Link>

          <nav className="hidden items-center gap-5 sm:flex">
            {user && (
              <>
                <NavLink to="/vote" className={navClass}>
                  Vote
                </NavLink>
                <NavLink to="/ranking" className={navClass}>
                  Your chart
                </NavLink>
              </>
            )}
            <NavLink to="/chart" className={navClass}>
              Chart
            </NavLink>
            {user?.is_admin && (
              <NavLink to="/admin" className={navClass}>
                Admin
              </NavLink>
            )}
          </nav>
        </div>

        {loading ? (
          <span className="h-8 w-24 animate-pulse bg-ink/10" />
        ) : user ? (
          <div className="flex items-center gap-3">
            <span className="hidden items-center gap-2 font-mono text-[10px] uppercase tracking-[0.16em] text-muted sm:flex">
              <span className="size-1.5 bg-ok" />
              {user.username}
              {user.is_admin && (
                <span className="border border-accent/40 px-1.5 py-0.5 text-[9px] text-accent">
                  Admin
                </span>
              )}
            </span>
            <button type="button" onClick={() => void handleLogout()} className="btn-ghost">
              Log out
            </button>
          </div>
        ) : (
          <a href="/#auth" className="btn-ghost">
            Membership
          </a>
        )}
      </div>

      <nav className="flex gap-5 border-t border-line px-5 py-2.5 sm:hidden">
        {user && (
          <>
            <NavLink to="/vote" className={navClass}>
              Vote
            </NavLink>
            <NavLink to="/ranking" className={navClass}>
              Your chart
            </NavLink>
          </>
        )}
        <NavLink to="/chart" className={navClass}>
          Chart
        </NavLink>
        {user?.is_admin && (
          <NavLink to="/admin" className={navClass}>
            Admin
          </NavLink>
        )}
      </nav>
    </header>
  )
}
