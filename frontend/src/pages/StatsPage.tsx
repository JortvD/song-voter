import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'

import { Countdown } from '../components/Countdown'
import { SeasonUnavailable } from '../components/SeasonUnavailable'
import { StoryCardGrid, StoryLeaderboards } from '../components/StoryCards'
import { useAuth } from '../context/AuthContext'
import { usePlaylist } from '../context/PlaylistContext'
import { ApiError, api } from '../lib/api'
import type {
  AggregateEntry,
  ChartStories,
  ChartStatus,
  FacetCount,
  User,
} from '../lib/types'

const LEADERBOARD_SIZE = 10

function formatNumber(value: number) {
  return value.toLocaleString()
}

function formatWhen(value: string | null) {
  if (!value) return 'soon'
  return new Date(value).toLocaleString(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  })
}

function StatPill({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border-2 border-line bg-paper px-3 py-2">
      <p className="font-mono text-[8px] uppercase tracking-[0.14em] text-muted">
        {label}
      </p>
      <p className="mt-1 font-serif text-xl leading-none">{value}</p>
    </div>
  )
}

function StatCover({
  image,
  title,
  size = 'size-10',
}: {
  image: string | null
  title: string
  size?: string
}) {
  if (image) {
    return (
      <img
        src={image}
        alt={`${title} cover`}
        loading="lazy"
        className={`${size} shrink-0 rounded-lg border-2 border-ink object-cover`}
      />
    )
  }

  return (
    <span
      className={`${size} grid shrink-0 place-items-center rounded-lg border-2 border-ink bg-paper font-serif text-lg text-muted/60`}
    >
      ♪
    </span>
  )
}

function FacetChips({
  label,
  allLabel,
  options,
  value,
  onChange,
}: {
  label: string
  allLabel: string
  options: { value: string; label: string; count: number }[]
  value: string | null
  onChange: (value: string | null) => void
}) {
  if (options.length === 0) return null

  const chip = (active: boolean) =>
    `rounded-full border-2 px-2.5 py-1 font-mono text-[9px] uppercase tracking-[0.1em] transition ${
      active
        ? 'border-ink bg-[var(--playlist-accent)] text-[var(--playlist-accent-ink)]'
        : 'border-line bg-paper text-muted hover:border-ink hover:text-ink'
    }`

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <span className="mr-1 font-mono text-[8px] uppercase tracking-[0.14em] text-muted">
        {label}
      </span>
      <button
        type="button"
        onClick={() => onChange(null)}
        aria-pressed={value === null}
        className={chip(value === null)}
      >
        {allLabel}
      </button>
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          onClick={() => onChange(value === option.value ? null : option.value)}
          aria-pressed={value === option.value}
          className={chip(value === option.value)}
        >
          {option.label}{' '}
          <span>{formatNumber(option.count)}</span>
        </button>
      ))}
    </div>
  )
}

function AggregateList({
  loading,
  title,
  entries,
  total,
  empty,
  error,
  onRetry,
}: {
  loading: boolean
  title: string
  entries: AggregateEntry[]
  total: number
  empty: string
  error?: string | null
  onRetry?: () => void
}) {
  return (
    <article className="pv-card min-w-0 p-4">
      <div className="flex min-w-0 flex-wrap items-end justify-between gap-2 border-b-2 border-ink/10 pb-2">
        <div className="min-w-0">
          <p className="pv-label">By average score</p>
          <h3 className="mt-1 break-words font-serif text-xl tracking-[-0.02em]">
            {title}
          </h3>
        </div>
        <span className="shrink-0 pv-label">
          Top {Math.min(entries.length, LEADERBOARD_SIZE)} of {formatNumber(total)}
        </span>
      </div>

      {error ? (
        <div role="alert" className="py-6 text-center">
          <p className="text-sm text-danger">{error}</p>
          {onRetry && (
            <button type="button" onClick={onRetry} className="btn-ghost mt-3">
              Try again
            </button>
          )}
        </div>
      ) : loading ? (
        <div className="mt-3 space-y-2">
          {[0, 1, 2, 3, 4].map((index) => (
            <div key={index} className="h-12 animate-pulse rounded-lg border border-line" />
          ))}
        </div>
      ) : entries.length === 0 ? (
        <p className="py-6 text-center text-sm text-muted">{empty}</p>
      ) : (
        <ol className="divide-y divide-line">
          {entries.map((entry, index) => (
            <li key={entry.id} className="flex min-w-0 items-center gap-2.5 py-2.5">
              <span className="w-5 shrink-0 font-serif text-lg leading-none text-muted">
                {String(index + 1).padStart(2, '0')}
              </span>
              <StatCover image={entry.image_url} title={entry.name} size="size-9" />
              <div className="min-w-0 flex-1">
                <p className="truncate text-xs font-medium text-ink">{entry.name}</p>
                <p className="truncate font-mono text-[8px] uppercase tracking-[0.1em] text-muted">
                  {entry.subtitle ?? entry.genre ?? 'Unknown artist'}
                </p>
              </div>
              <div className="max-w-20 shrink-0 text-right">
                <p className="font-serif text-sm leading-none text-ink">
                  {Math.round(entry.rating)}
                </p>
                <p className="truncate font-mono text-[8px] uppercase tracking-[0.1em] text-muted">
                  {formatNumber(entry.track_count)} songs
                </p>
              </div>
              <div className="hidden w-14 shrink-0 text-right sm:block">
                <p className="font-mono text-[8px] uppercase tracking-[0.1em] text-muted">
                  duels
                </p>
                <p className="font-serif text-sm leading-none text-ink">
                  {formatNumber(entry.comparisons)}
                </p>
              </div>
            </li>
          ))}
        </ol>
      )}
    </article>
  )
}

function Timeline({
  eyebrow,
  label,
  options,
  selected,
  onSelect,
  formatValue,
}: {
  eyebrow: string
  label: string
  options: FacetCount[]
  selected: string | null
  onSelect: (value: string | null) => void
  formatValue: (value: string) => string
}) {
  if (options.length === 0) return null

  const max = Math.max(...options.map((option) => option.count), 1)
  const chip = (active: boolean) =>
    `w-full rounded-lg px-2 py-1.5 text-left transition ${
      active ? 'bg-[var(--playlist-accent)]/15' : 'hover:bg-paper'
    }`

  return (
    <article className="pv-card min-w-0 p-4">
      <div className="flex min-w-0 flex-wrap items-end justify-between gap-2 border-b-2 border-ink/10 pb-2">
        <div className="min-w-0">
          <p className="pv-label">{eyebrow}</p>
          <h3 className="mt-1 font-serif text-xl tracking-[-0.02em]">{label}</h3>
        </div>
        <span className="pv-label">Whole tape · tap to filter</span>
      </div>

      <div className="mt-3 space-y-1">
        {options.map((option) => {
          const active = selected === option.value
          return (
            <button
              key={option.value}
              type="button"
              onClick={() => onSelect(active ? null : option.value)}
              aria-pressed={active}
              className={chip(active)}
            >
              <span className="grid grid-cols-[3.5rem_1fr_2rem_2rem] items-center gap-2 sm:grid-cols-[4.5rem_1fr_2.5rem_2.5rem]">
                <span className="truncate font-mono text-[9px] uppercase tracking-[0.1em] text-muted">
                  {formatValue(option.value)}
                </span>
                <span className="min-w-0 h-2 overflow-hidden rounded-full bg-line">
                  <span
                    className="block h-full rounded-full bg-[var(--playlist-accent)]"
                    style={{ width: `${Math.max(6, (option.count / max) * 100)}%` }}
                  />
                </span>
                <span className="text-right font-mono text-[9px] text-muted">
                  {formatNumber(option.count)}
                </span>
                <span className="text-right font-mono text-[9px] text-ink">
                  {option.average_rating === null
                    ? '—'
                    : Math.round(option.average_rating)}
                </span>
              </span>
            </button>
          )
        })}
      </div>
      <p className="mt-2 font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
        Song count · rated average
      </p>
    </article>
  )
}

function SealedStats({ status, user }: { status: ChartStatus; user: User | null }) {
  const scheduled = status.started_at === null

  return (
    <>
      <div className="flex min-w-0 flex-wrap items-end justify-between gap-3 border-b-2 border-ink/15 pb-4">
        <div className="min-w-0">
          <p className="pv-label">Public</p>
          <h1 className="mt-1 break-words font-serif text-4xl leading-none tracking-[-0.04em] sm:text-5xl">
            {scheduled ? 'The stats are on deck.' : 'The Stats are sealed.'}
          </h1>
          <p className="mt-2 break-words font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {status.playlist_name} · {formatNumber(status.votes_cast)} picks ·{' '}
            {formatNumber(status.total_tracks)} songs
          </p>
        </div>
      </div>

      <section className="pv-card mt-6 p-6 sm:p-8">
        <div className="flex flex-wrap gap-2">
          <span className={`pv-sticker ${scheduled ? 'pv-sticker-butter' : ''}`}>
            {scheduled ? 'Scheduled' : 'Sealed'}
          </span>
          {status.voting_paused && (
            <span className="pv-sticker pv-sticker-paper">Voting paused</span>
          )}
        </div>
        <h2 className="mt-4 font-serif text-3xl leading-tight tracking-[-0.03em]">
          {scheduled
            ? `Voting opens ${formatWhen(status.scheduled_start_at)}.`
            : 'The numbers arrive with The Reveal.'}
        </h2>
        <p className="mt-3 max-w-xl text-sm leading-relaxed text-muted">
          {scheduled
            ? 'The tape is set, but the songs are fetched at the start. This page will fill with artist and album leaderboards, release timelines and liner notes once the tape opens and The Reveal follows.'
            : 'The group\u2019s stats are being built in private. Once the admin opens The Reveal, this page fills with artist and album leaderboards, release timelines, liner notes and the full story of the tape.'}
        </p>
        {status.scheduled_reveal_at && (
          <Countdown
            target={status.scheduled_reveal_at}
            label="The seal opens in"
          />
        )}
        <div className="mt-5 grid grid-cols-3 gap-2">
          <StatPill label="Picks" value={formatNumber(status.votes_cast)} />
          <StatPill label="Songs" value={formatNumber(status.total_tracks)} />
          <StatPill label="Voters" value={formatNumber(status.voters)} />
        </div>
        <div className="mt-6 flex flex-wrap gap-3">
          <Link to="/standings" className="btn-ghost">
            The Standings
          </Link>
          {!scheduled && user && (
            <Link to="/duel" className="btn">
              Next duel
            </Link>
          )}
        </div>
      </section>
    </>
  )
}

function NoPlaylistStats({ user }: { user: User | null }) {
  return (
    <section className="pv-card p-6 text-center sm:p-10">
      <span className="pv-sticker pv-sticker-paper">No tape yet</span>
      <h1 className="mt-4 font-serif text-4xl leading-tight tracking-[-0.03em]">
        There are no stats yet.
      </h1>
      <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
        The admin hasn&apos;t imported a playlist yet, so there is nothing to
        measure.
      </p>
      <div className="mt-6 flex flex-wrap justify-center gap-3">
        {user?.is_admin && (
          <Link to="/admin" className="btn">
            Import a tape
          </Link>
        )}
        <Link to="/" className="btn-ghost">
          Back home
        </Link>
      </div>
    </section>
  )
}

export function StatsPage() {
  const { user } = useAuth()
  const {
    status,
    loading: playlistLoading,
    error: playlistError,
    refresh: refreshPlaylist,
  } = usePlaylist()
  const [stories, setStories] = useState<ChartStories | null>(null)
  const [artists, setArtists] = useState<AggregateEntry[]>([])
  const [albums, setAlbums] = useState<AggregateEntry[]>([])
  const [artistTotal, setArtistTotal] = useState(0)
  const [albumTotal, setAlbumTotal] = useState(0)
  const [artistId, setArtistId] = useState<string | null>(null)
  const [decade, setDecade] = useState<number | null>(null)
  const [genre, setGenre] = useState<string | null>(null)
  const [statsLoading, setStatsLoading] = useState(true)
  const [storiesLoading, setStoriesLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [statsError, setStatsError] = useState<string | null>(null)
  const [reloadKey, setReloadKey] = useState(0)

  useEffect(() => {
    if (!status?.revealed) {
      setStories(null)
      setStoriesLoading(false)
      return
    }

    let active = true
    const controller = new AbortController()
    setStoriesLoading(true)
    setStories(null)

    api
      .getChartStories(controller.signal)
      .then((data) => {
        if (active) setStories(data)
      })
      .catch((caught: unknown) => {
        if (!active || controller.signal.aborted) return
        setError(
          caught instanceof ApiError
            ? caught.message
            : 'Could not load the liner notes',
        )
      })
      .finally(() => {
        if (active) setStoriesLoading(false)
      })

    return () => {
      active = false
      controller.abort()
    }
  }, [status?.revealed, status?.playlist_id, reloadKey])

  useEffect(() => {
    if (!status?.revealed) {
      setStatsLoading(false)
      return
    }

    let active = true
    const controller = new AbortController()
    setStatsLoading(true)
    setStatsError(null)

    const filters = {
      limit: LEADERBOARD_SIZE,
      artist: artistId,
      decade,
      genre,
    }

    Promise.all([
      api.getChartArtists(filters, controller.signal),
      api.getChartAlbums(filters, controller.signal),
    ])
      .then(([artistData, albumData]) => {
        if (!active) return
        setArtists(artistData.entries)
        setArtistTotal(artistData.total)
        setAlbums(albumData.entries)
        setAlbumTotal(albumData.total)
      })
      .catch((caught: unknown) => {
        if (!active || controller.signal.aborted) return
        if (caught instanceof ApiError && caught.status === 403) {
          setError('The standings were sealed again. Refreshing…')
          void refreshPlaylist()
        } else {
          setStatsError(
            caught instanceof ApiError
              ? caught.message
              : 'Could not load the stats',
          )
        }
      })
      .finally(() => {
        if (active) setStatsLoading(false)
      })

    return () => {
      active = false
      controller.abort()
    }
  }, [
    status?.revealed,
    status?.playlist_id,
    artistId,
    decade,
    genre,
    reloadKey,
    refreshPlaylist,
  ])

  function refresh() {
    setReloadKey((value) => value + 1)
    void refreshPlaylist()
  }

  function clearFilters() {
    setArtistId(null)
    setDecade(null)
    setGenre(null)
  }

  if (playlistLoading && !status) {
    return (
      <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-6 sm:px-6 sm:py-8 lg:px-8">
        <div className="h-28 animate-pulse rounded-2xl border-2 border-line bg-card" />
        <div className="mt-6 grid gap-2 sm:grid-cols-3">
          {[0, 1, 2, 3, 4, 5].map((index) => (
            <div
              key={index}
              className="h-20 animate-pulse rounded-xl border-2 border-line bg-card"
            />
          ))}
        </div>
      </main>
    )
  }

  if (!status && playlistError) {
    return (
      <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-8 sm:px-6 sm:py-10 lg:px-8">
        <SeasonUnavailable
          error={playlistError}
          onRetry={refreshPlaylist}
          title="The stats are unavailable."
        />
      </main>
    )
  }

  if (!status?.playlist_name) {
    return (
      <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-8 sm:px-6 sm:py-10 lg:px-8">
        <NoPlaylistStats user={user} />
      </main>
    )
  }

  if (!status.revealed) {
    return (
      <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-6 sm:px-6 sm:py-8 lg:px-8">
        <SealedStats status={status} user={user} />
      </main>
    )
  }

  const hasFilters = Boolean(artistId || decade !== null || genre)
  const artistFacets = status.artists ?? []
  const decadeFacets = status.decades ?? []
  const genreFacets = status.genres ?? []
  const coverage =
    status.total_tracks > 0
      ? Math.round((status.tracks_compared / status.total_tracks) * 100)
      : 0
  const years =
    status.year_min !== null && status.year_max !== null
      ? `${status.year_min}–${status.year_max}`
      : '—'

  return (
    <main className="pv-rise mx-auto w-full max-w-6xl flex-1 px-4 py-6 sm:px-6 sm:py-8 lg:px-8">
      <header className="border-b-2 border-ink/15 pb-4">
        <div className="flex min-w-0 flex-wrap items-end justify-between gap-3">
          <div className="min-w-0">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <span className="pv-label">Public</span>
              <span className="pv-sticker pv-sticker-cobalt">The numbers</span>
              <span className="max-w-full break-words pv-label">
                {status.playlist_name}
              </span>
            </div>
            <h1 className="mt-2 break-words font-serif text-4xl leading-none tracking-[-0.04em] sm:text-5xl">
              The Stats.
            </h1>
            <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              {formatNumber(status.total_tracks)} songs ·{' '}
              {formatNumber(status.total_artists)} artists ·{' '}
              {formatNumber(status.total_albums)} albums · {formatNumber(status.duels)}{' '}
              duels
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Link to="/standings" className="btn-ghost">
              Standings
            </Link>
            <button type="button" onClick={refresh} className="btn-ghost">
              Refresh
            </button>
            {user && (
              <Link to="/duel" className="btn">
                Duel
              </Link>
            )}
          </div>
        </div>
      </header>

      <section className="mt-5 grid grid-cols-2 gap-2 sm:grid-cols-4 lg:grid-cols-7">
        <StatPill label="Songs" value={formatNumber(status.total_tracks)} />
        <StatPill label="Artists" value={formatNumber(status.total_artists)} />
        <StatPill label="Albums" value={formatNumber(status.total_albums)} />
        <StatPill label="Duels" value={formatNumber(status.duels)} />
        <StatPill label="Picks" value={formatNumber(status.votes_cast)} />
        <StatPill label="Voters" value={formatNumber(status.voters)} />
        <StatPill label="Coverage" value={`${coverage}%`} />
      </section>

      <nav
        aria-label="Stats sections"
        className="mt-4 flex flex-wrap items-center gap-x-4 gap-y-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted"
      >
        {stories && stories.cards.length > 0 && (
          <a href="#liner-notes" className="underline decoration-line underline-offset-2 hover:text-ink">
            Liner notes
          </a>
        )}
        {stories && stories.leaderboards.length > 0 && (
          <a href="#leaderboards" className="underline decoration-line underline-offset-2 hover:text-ink">
            Top 5
          </a>
        )}
        <a href="#explore" className="underline decoration-line underline-offset-2 hover:text-ink">
          Explore
        </a>
      </nav>

      {error && (
        <p
          role="alert"
          className="mt-4 rounded-xl border-2 border-danger bg-danger/5 px-3 py-2 text-sm text-danger"
        >
          {error}
        </p>
      )}

      {storiesLoading ? (
        <div className="mt-8 grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {[0, 1, 2].map((index) => (
            <div
              key={index}
              className="h-56 animate-pulse rounded-2xl border-2 border-line bg-card"
            />
          ))}
        </div>
      ) : stories && stories.cards.length > 0 ? (
        <section id="liner-notes" className="mt-8 scroll-mt-24">
          <div className="flex flex-wrap items-end justify-between gap-3 border-b-2 border-ink/15 pb-2">
            <div>
              <p className="pv-label">The stories</p>
              <h2 className="mt-1 font-serif text-2xl tracking-[-0.02em]">
                Liner notes
              </h2>
            </div>
            <span className="pv-label">Across the whole tape</span>
          </div>
          <StoryCardGrid cards={stories.cards} />
        </section>
      ) : null}

      {stories && stories.leaderboards.length > 0 && (
        <section id="leaderboards" className="mt-8 scroll-mt-24">
          <div className="flex flex-wrap items-end justify-between gap-3 border-b-2 border-ink/15 pb-2">
            <div>
              <p className="pv-label">The top 5</p>
              <h2 className="mt-1 font-serif text-2xl tracking-[-0.02em]">
                Leaderboards
              </h2>
            </div>
            <span className="pv-label">The names at the top</span>
          </div>
          <StoryLeaderboards leaderboards={stories.leaderboards} />
        </section>
      )}

      {!storiesLoading && (
        <section id="explore" className="mt-9 scroll-mt-24">
        <div className="flex flex-wrap items-end justify-between gap-3 border-b-2 border-ink/15 pb-2">
          <div>
            <p className="pv-label">The deep cuts</p>
            <h2 className="mt-1 font-serif text-2xl tracking-[-0.02em]">
              Explore the tape
            </h2>
          </div>
          <span className="pv-label">
            {status.year_min !== null && status.year_max !== null
              ? `Released ${years}`
              : 'Filter by artist and era'}
          </span>
        </div>

        <div className="pv-card mt-4 min-w-0 p-4">
          <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
            <label className="block min-w-0">
              <span className="pv-label mb-1.5 block">Artist</span>
              <select
                value={artistId ?? ''}
                onChange={(event) => setArtistId(event.target.value || null)}
                className="pv-input w-full min-w-0 max-w-full"
              >
                <option value="">All artists</option>
                {artistFacets.map((artist) => (
                  <option key={artist.id} value={artist.id}>
                    {artist.name} ({formatNumber(artist.count)})
                  </option>
                ))}
              </select>
            </label>

            <div className="flex flex-wrap items-center gap-2 sm:justify-end">
              <span className="font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                {statsLoading
                  ? 'Filtering…'
                  : `${formatNumber(artistTotal)} artists · ${formatNumber(albumTotal)} albums`}
              </span>
              {hasFilters && (
                <button type="button" onClick={clearFilters} className="btn-ghost">
                  Clear filters
                </button>
              )}
            </div>
          </div>

          <div className="mt-3 space-y-2">
            <FacetChips
              label="When the track was made"
              allLabel="All eras"
              value={decade === null ? null : String(decade)}
              options={decadeFacets.map((facet) => ({
                value: facet.value,
                label: `${facet.value}s`,
                count: facet.count,
              }))}
              onChange={(value) => setDecade(value === null ? null : Number(value))}
            />
            {genreFacets.length > 0 && (
              <FacetChips
                label="Genre"
                allLabel="All genres"
                value={genre}
                options={genreFacets.map((facet) => ({
                  value: facet.value,
                  label: facet.value,
                  count: facet.count,
                }))}
                onChange={setGenre}
              />
            )}
          </div>
          <p className="mt-2 font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
            Artist and album averages use songs with at least one duel.
          </p>
        </div>

        <div className="mt-4 grid gap-4 lg:grid-cols-2">
          <AggregateList
            loading={statsLoading}
            title="Artists"
            entries={artists}
            total={artistTotal}
            empty="No artists match these filters."
            error={statsError}
            onRetry={refresh}
          />
          <AggregateList
            loading={statsLoading}
            title="Albums"
            entries={albums}
            total={albumTotal}
            empty="No albums match these filters."
            error={statsError}
            onRetry={refresh}
          />
        </div>

        <div className="mt-4 grid gap-4 lg:grid-cols-2">
          <Timeline
            eyebrow="When the track was made"
            label="Decade explorer"
            options={decadeFacets}
            selected={decade === null ? null : String(decade)}
            onSelect={(value) => setDecade(value === null ? null : Number(value))}
            formatValue={(value) => `${value}s`}
          />
          {genreFacets.length > 0 && (
            <Timeline
              eyebrow="What it sounds like"
              label="Genre explorer"
              options={genreFacets}
              selected={genre}
              onSelect={setGenre}
              formatValue={(value) => value}
            />
          )}
        </div>
        </section>
      )}
    </main>
  )
}
