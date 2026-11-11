import type { AdminInsights } from '../../lib/types'

export type AdminSection =
  | 'overview'
  | 'duels'
  | 'voters'
  | 'standings'
  | 'pairing'
  | 'playlist'
  | 'reveal'
  | 'invites'
  | 'settings'
  | 'runtime'
  | 'danger'

type Props = {
  section: AdminSection
  onChange: (section: AdminSection) => void
  insights: AdminInsights | null
}

const items: Array<{
  id: AdminSection
  label: string
  hint: (insights: AdminInsights | null) => string
}> = [
  { id: 'overview', label: 'Overview', hint: () => 'The whole room' },
  {
    id: 'duels',
    label: 'Duels',
    hint: (insights) => (insights ? `${insights.total_duels} shown` : '—'),
  },
  {
    id: 'voters',
    label: 'Voters',
    hint: (insights) => (insights ? `${insights.total_users} accounts` : '—'),
  },
  {
    id: 'standings',
    label: 'Standings',
    hint: (insights) => (insights ? `${insights.total_tracks} songs` : '—'),
  },
  {
    id: 'pairing',
    label: 'Pairing health',
    hint: () => 'Weighting',
  },
  {
    id: 'playlist',
    label: 'Playlist',
    hint: (insights) => (insights ? `${insights.previews_missing} gaps` : '—'),
  },
  {
    id: 'reveal',
    label: 'The Reveal',
    hint: (insights) => (insights?.revealed ? 'Live' : 'Sealed'),
  },
  {
    id: 'invites',
    label: 'Invites',
    hint: (insights) => (insights ? `${insights.invites_open} open` : '—'),
  },
  { id: 'settings', label: 'Settings', hint: () => 'League rules' },
  { id: 'runtime', label: 'Runtime', hint: () => 'Health' },
  { id: 'danger', label: 'Danger zone', hint: () => 'Careful' },
]

export function AdminNav({ section, onChange, insights }: Props) {
  return (
    <nav
      aria-label="Mission control sections"
      className="flex gap-2 overflow-x-auto pb-2 lg:sticky lg:top-20 lg:flex-col lg:self-start lg:overflow-visible lg:pb-0"
    >
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          onClick={() => onChange(item.id)}
          className={`pv-admin-nav-item whitespace-nowrap ${
            section === item.id ? 'pv-admin-nav-item-active' : ''
          }`}
        >
          <span>{item.label}</span>
          <span
            className={`hidden text-[8px] normal-case tracking-normal sm:inline ${
              section === item.id ? 'text-paper/70' : 'text-muted'
            }`}
          >
            {item.hint(insights)}
          </span>
        </button>
      ))}
    </nav>
  )
}
