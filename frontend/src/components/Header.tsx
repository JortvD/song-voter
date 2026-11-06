import { Link, NavLink, useLocation, useNavigate } from 'react-router-dom'

import { BrandMark } from './BrandMark'
import { PlaylistBadge } from './PlaylistBadge'
import { useAuth } from '../context/AuthContext'
import { usePlaylist } from '../context/PlaylistContext'

const navClass = ({ isActive }: { isActive: boolean }) =>
  `pv-tab ${isActive ? 'pv-tab-active' : ''}`

export function Header() {
  const { user, loading, logout } = useAuth()
  const { status, loading: playlistLoading } = usePlaylist()
  const navigate = useNavigate()
  const location = useLocation()
  const duelFocus = location.pathname.startsWith('/duel')

  async function handleLogout() {
    await logout()
    navigate('/')
  }

  return (
    <header className="sticky top-0 z-40 border-b-2 border-ink/15 bg-paper/95 backdrop-blur">
      <div className="h-1.5 bg-[var(--playlist-accent)]" />

      <div
        className={`mx-auto flex w-full max-w-6xl items-center gap-3 px-4 sm:gap-4 sm:px-6 lg:px-8 ${
          duelFocus ? 'py-1 lg:py-2.5' : 'py-2.5'
        }`}
      >
        <Link to="/" className="flex shrink-0 items-center gap-2.5">
          <BrandMark className={duelFocus ? 'size-6 lg:size-9' : 'size-8 sm:size-9'} />
          <span
            className={`font-serif text-lg leading-none tracking-[-0.03em] ${
              duelFocus ? 'hidden lg:block' : 'hidden sm:block'
            }`}
          >
            Playlist Voter
          </span>
        </Link>

        <span className="hidden h-8 w-px bg-ink/15 lg:block" />

        <div className={`min-w-0 flex-1 ${duelFocus ? 'hidden lg:block' : ''}`}>
          <PlaylistBadge status={status} loading={playlistLoading} />
        </div>

        {duelFocus && (
          <span className="flex min-w-0 flex-1 items-center gap-2 font-mono text-[9px] font-semibold uppercase tracking-[0.16em] text-muted lg:hidden">
            <span className="size-1.5 shrink-0 rounded-full bg-[var(--playlist-accent)]" />
            <span className="truncate">
              Duel{status?.playlist_name ? ` · ${status.playlist_name}` : ''}
            </span>
          </span>
        )}

        <nav className="hidden items-center gap-0.5 lg:flex">
          {user && (
            <>
              <NavLink to="/duel" className={navClass}>
                Duel
              </NavLink>
              <NavLink to="/your-ranking" className={navClass}>
                Your Ranking
              </NavLink>
            </>
          )}
          <NavLink to="/standings" className={navClass}>
            Standings
          </NavLink>
          <NavLink to="/stats" className={navClass}>
            Stats
          </NavLink>
          {user?.is_admin && (
            <NavLink to="/admin" className={navClass}>
              Admin
            </NavLink>
          )}
        </nav>

        <div className="hidden items-center gap-2.5 lg:flex">
          {loading ? (
            <span className="h-7 w-24 animate-pulse rounded-full bg-ink/10" />
          ) : user ? (
            <>
              <span className="flex items-center gap-1.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
                <span className="size-1.5 rounded-full bg-ok" />
                <span className="max-w-24 truncate">{user.username}</span>
                {user.is_admin && (
                  <span className="rounded border border-cobalt/40 px-1 py-0.5 text-[9px] text-cobalt">
                    Admin
                  </span>
                )}
              </span>
              <button
                type="button"
                onClick={() => void handleLogout()}
                className="rounded-full border-2 border-ink px-3 py-1.5 font-mono text-[9px] font-semibold uppercase tracking-[0.12em] transition hover:bg-card"
              >
                Log out
              </button>
            </>
          ) : (
            <a
              href="/#auth"
              className="rounded-full border-2 border-ink px-4 py-2 font-mono text-[9px] font-semibold uppercase tracking-[0.14em] transition hover:bg-card"
            >
              Membership
            </a>
          )}
        </div>

        <div className={`items-center lg:hidden ${duelFocus ? 'hidden' : 'flex'}`}>
          {loading ? (
            <span className="h-7 w-7 animate-pulse rounded-full bg-ink/10" />
          ) : user ? (
            <button
              type="button"
              onClick={() => void handleLogout()}
              className="rounded-full border-2 border-ink px-2 py-1.5 font-mono text-[9px] font-semibold uppercase tracking-[0.08em]"
              aria-label="Log out"
            >
              Log out
            </button>
          ) : (
            <a
              href="/#auth"
              className="rounded-full border-2 border-ink px-2 py-1.5 font-mono text-[9px] font-semibold uppercase tracking-[0.08em]"
            >
              Sign in
            </a>
          )}
        </div>
      </div>
    </header>
  )
}
