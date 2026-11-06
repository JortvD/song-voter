import { NavLink } from 'react-router-dom'

import { useAuth } from '../context/AuthContext'

type IconName =
  | 'home'
  | 'duel'
  | 'ranking'
  | 'standings'
  | 'stats'
  | 'admin'
  | 'join'

function TabIcon({ name }: { name: IconName }) {
  const common = {
    viewBox: '0 0 24 24',
    className: 'size-4',
    fill: 'none',
    stroke: 'currentColor',
    strokeWidth: 1.8,
    strokeLinecap: 'round' as const,
    strokeLinejoin: 'round' as const,
    'aria-hidden': true,
  }

  if (name === 'home') {
    return (
      <svg {...common}>
        <rect x="3.5" y="6" width="17" height="12" rx="2.5" />
        <circle cx="8" cy="12" r="2" />
        <circle cx="16" cy="12" r="2" />
        <path d="M10 12h4" />
      </svg>
    )
  }

  if (name === 'duel') {
    return (
      <svg {...common}>
        <rect x="3.5" y="5.5" width="7.5" height="13" rx="2" transform="rotate(-6 7.25 12)" />
        <rect x="13" y="5.5" width="7.5" height="13" rx="2" transform="rotate(6 16.75 12)" />
        <path d="M12 8v8" />
      </svg>
    )
  }

  if (name === 'ranking') {
    return (
      <svg {...common}>
        <path d="M5 6h14M5 12h10M5 18h7" />
        <path d="M17 16l2 2 3-3" />
      </svg>
    )
  }

  if (name === 'standings') {
    return (
      <svg {...common}>
        <path d="M4 19h16" />
        <rect x="5" y="11" width="4" height="7" rx="1" />
        <rect x="10" y="7" width="4" height="11" rx="1" />
        <rect x="15" y="13" width="4" height="5" rx="1" />
      </svg>
    )
  }

  if (name === 'stats') {
    return (
      <svg {...common}>
        <path d="M4 19h16" />
        <path d="M5 15l4-4 3 2 4-6 3 3" />
        <circle cx="9" cy="11" r="1" fill="currentColor" stroke="none" />
        <circle cx="16" cy="7" r="1" fill="currentColor" stroke="none" />
      </svg>
    )
  }

  if (name === 'admin') {
    return (
      <svg {...common}>
        <path d="M4 7h10M18 7h2M4 17h2M10 17h10" />
        <circle cx="16" cy="7" r="2" />
        <circle cx="8" cy="17" r="2" />
      </svg>
    )
  }

  return (
    <svg {...common}>
      <rect x="3.5" y="5" width="17" height="14" rx="2.5" />
      <path d="M8 5v14" />
      <path d="M12 9h5M12 13h3" />
    </svg>
  )
}

function tabClass({ isActive }: { isActive: boolean }) {
  return `group flex min-w-0 flex-1 flex-col items-center gap-1 ${
    isActive ? 'text-ink' : 'text-muted'
  }`
}

function TabBody({
  name,
  label,
  active,
}: {
  name: IconName
  label: string
  active: boolean
}) {
  return (
    <>
      <span
        className={`grid size-8 place-items-center rounded-full border-2 transition ${
          active
            ? 'border-ink bg-[var(--playlist-accent)] text-[var(--playlist-accent-ink)] shadow-[2px_2px_0_0_#141a2a]'
            : 'border-transparent'
        }`}
      >
        <TabIcon name={name} />
      </span>
      <span className="font-mono text-[9px] font-semibold uppercase tracking-[0.12em]">
        {label}
      </span>
    </>
  )
}

export function BottomTabs() {
  const { user } = useAuth()

  return (
    <nav
      aria-label="Primary"
      className="fixed inset-x-0 bottom-0 z-40 border-t-2 border-ink/15 bg-paper/95 px-2 pb-[max(0.45rem,env(safe-area-inset-bottom))] pt-1.5 backdrop-blur lg:hidden"
    >
      <div className="mx-auto flex max-w-md items-stretch gap-1">
        <NavLink to="/" className={tabClass} end>
          {({ isActive }) => <TabBody name="home" label="Home" active={isActive} />}
        </NavLink>

        {user ? (
          <>
            <NavLink to="/duel" className={tabClass}>
              {({ isActive }) => <TabBody name="duel" label="Duel" active={isActive} />}
            </NavLink>
            <NavLink to="/your-ranking" className={tabClass}>
              {({ isActive }) => (
                <TabBody name="ranking" label="Ranking" active={isActive} />
              )}
            </NavLink>
            <NavLink to="/standings" className={tabClass}>
              {({ isActive }) => (
                <TabBody name="standings" label="Standings" active={isActive} />
              )}
            </NavLink>
            <NavLink to="/stats" className={tabClass}>
              {({ isActive }) => (
                <TabBody name="stats" label="Stats" active={isActive} />
              )}
            </NavLink>
            {user.is_admin && (
              <NavLink to="/admin" className={tabClass}>
                {({ isActive }) => (
                  <TabBody name="admin" label="Admin" active={isActive} />
                )}
              </NavLink>
            )}
          </>
        ) : (
          <>
            <NavLink to="/standings" className={tabClass}>
              {({ isActive }) => (
                <TabBody name="standings" label="Standings" active={isActive} />
              )}
            </NavLink>
            <NavLink to="/stats" className={tabClass}>
              {({ isActive }) => (
                <TabBody name="stats" label="Stats" active={isActive} />
              )}
            </NavLink>
            <a href="/#auth" className="flex min-w-0 flex-1 flex-col items-center gap-1 text-muted">
              <TabBody name="join" label="Join" active={false} />
            </a>
          </>
        )}
      </div>
    </nav>
  )
}
