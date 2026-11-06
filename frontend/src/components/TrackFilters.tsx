import { useState } from 'react'

import type { ArtistFacet, FacetCount } from '../lib/types'

type TrackFiltersProps = {
  ariaLabel?: string
  searchLabel?: string
  searchPlaceholder?: string
  search: string
  onSearchChange: (value: string) => void
  artists: ArtistFacet[]
  artistId: string | null
  onArtistChange: (value: string | null) => void
  decades: FacetCount[]
  decade: number | null
  onDecadeChange: (value: number | null) => void
  genres: FacetCount[]
  genre: string | null
  onGenreChange: (value: string | null) => void
  resultCount: number
  totalCount: number
  loading: boolean
  onClear: () => void
}

function chipClass(active: boolean) {
  return `rounded-full border-2 px-2 py-0.5 font-mono text-[9px] uppercase tracking-[0.1em] transition ${
    active
      ? 'border-ink bg-[var(--playlist-accent)] text-[var(--playlist-accent-ink)]'
      : 'border-line bg-paper text-muted hover:border-ink hover:text-ink'
  }`
}

function SearchIcon() {
  return (
    <svg
      viewBox="0 0 20 20"
      className="size-3.5"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <circle cx="8.5" cy="8.5" r="4.8" />
      <path d="m12.2 12.2 4 4" />
    </svg>
  )
}

function FilterIcon() {
  return (
    <svg
      viewBox="0 0 20 20"
      className="size-3.5"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <path d="M3.5 5.5h13M5.5 10h9M8 14.5h4" />
    </svg>
  )
}

export function TrackFilters({
  ariaLabel = 'Find a song in the list',
  searchLabel = 'Search the list',
  searchPlaceholder = 'Find a song, artist or album',
  search,
  onSearchChange,
  artists,
  artistId,
  onArtistChange,
  decades,
  decade,
  onDecadeChange,
  genres,
  genre,
  onGenreChange,
  resultCount,
  totalCount,
  loading,
  onClear,
}: TrackFiltersProps) {
  const [open, setOpen] = useState(false)

  const panelFilterCount =
    (artistId ? 1 : 0) + (decade !== null ? 1 : 0) + (genre ? 1 : 0)
  const hasAnyFilter = Boolean(search.trim() || panelFilterCount)
  const activeArtist = artists.find((artist) => artist.id === artistId)

  return (
    <section
      aria-label={ariaLabel}
      className="mt-4 rounded-xl border-2 border-ink/15 bg-card px-2.5 py-2"
    >
      <div className="flex flex-wrap items-center gap-2">
        <label className="relative min-w-0 flex-1 sm:max-w-sm">
          <span className="sr-only">{searchLabel}</span>
          <span className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-muted">
            <SearchIcon />
          </span>
          <input
            type="search"
            value={search}
            onChange={(event) => onSearchChange(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Escape') {
                onSearchChange('')
                event.currentTarget.blur()
              }
            }}
            placeholder={searchPlaceholder}
            autoComplete="off"
            spellCheck={false}
            className="w-full rounded-lg border-2 border-muted bg-paper py-1.5 pl-8 pr-8 text-sm text-ink outline-none transition placeholder:text-muted focus:border-ink [&::-webkit-search-cancel-button]:hidden"
          />
          {search && (
            <button
              type="button"
              onClick={() => onSearchChange('')}
              aria-label="Clear search"
              className="absolute right-1.5 top-1/2 grid size-5 -translate-y-1/2 place-items-center rounded-full text-muted transition hover:bg-ink/10 hover:text-ink"
            >
              ×
            </button>
          )}
        </label>

        <button
          type="button"
          onClick={() => setOpen((value) => !value)}
          aria-expanded={open}
          aria-controls="standings-filter-panel"
          className={`flex items-center gap-1.5 rounded-lg border-2 px-2.5 py-1.5 font-mono text-[9px] font-semibold uppercase tracking-[0.12em] transition ${
            panelFilterCount > 0
              ? 'border-ink bg-[var(--playlist-accent)] text-[var(--playlist-accent-ink)]'
              : 'border-line bg-paper text-ink hover:border-ink'
          }`}
        >
          <FilterIcon />
          Filters
          {panelFilterCount > 0 && (
            <span className="grid size-4 place-items-center rounded-full border border-ink bg-paper text-[8px] leading-none text-ink">
              {panelFilterCount}
            </span>
          )}
          <span
            className={`text-[10px] leading-none transition ${
              open ? 'rotate-180' : ''
            }`}
            aria-hidden="true"
          >
            ▾
          </span>
        </button>

        {hasAnyFilter && (
          <button
            type="button"
            onClick={onClear}
            className="font-mono text-[9px] font-semibold uppercase tracking-[0.12em] text-muted underline decoration-line underline-offset-4 transition hover:text-ink"
          >
            Clear
          </button>
        )}

        <span className="ml-auto font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
          {loading
            ? 'Searching…'
            : hasAnyFilter
              ? `${resultCount.toLocaleString()} of ${totalCount.toLocaleString()} matches`
              : `${totalCount.toLocaleString()} songs`}
        </span>
      </div>

      {panelFilterCount > 0 && !open && (
        <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
          {activeArtist && (
            <button
              type="button"
              onClick={() => onArtistChange(null)}
              className={chipClass(true)}
              aria-label={`Remove artist filter ${activeArtist.name}`}
            >
              {activeArtist.name} ×
            </button>
          )}
          {decade !== null && (
            <button
              type="button"
              onClick={() => onDecadeChange(null)}
              className={chipClass(true)}
              aria-label={`Remove decade filter ${decade}s`}
            >
              {decade}s ×
            </button>
          )}
          {genre && (
            <button
              type="button"
              onClick={() => onGenreChange(null)}
              className={chipClass(true)}
              aria-label={`Remove genre filter ${genre}`}
            >
              {genre} ×
            </button>
          )}
        </div>
      )}

      {open && (
        <div
          id="standings-filter-panel"
          className="mt-2 grid gap-2.5 border-t-2 border-ink/10 pt-2 md:grid-cols-[minmax(0,0.9fr)_minmax(0,1fr)_minmax(0,1fr)]"
        >
          <label className="block min-w-0">
            <span className="pv-label mb-1 block">Artist</span>
            <select
              value={artistId ?? ''}
              onChange={(event) => onArtistChange(event.target.value || null)}
              className="w-full min-w-0 rounded-lg border-2 border-line bg-paper px-2 py-1.5 text-sm text-ink outline-none transition focus:border-ink"
            >
              <option value="">All artists</option>
              {artists.map((artist) => (
                <option key={artist.id} value={artist.id}>
                  {artist.name} ({artist.count.toLocaleString()})
                </option>
              ))}
            </select>
          </label>

          {decades.length > 0 && (
            <div className="min-w-0">
              <p className="pv-label mb-1">Era</p>
              <div className="flex flex-wrap gap-1">
                <button
                  type="button"
                  onClick={() => onDecadeChange(null)}
                  aria-pressed={decade === null}
                  className={chipClass(decade === null)}
                >
                  All
                </button>
                {decades.map((facet) => {
                  const value = Number(facet.value)
                  return (
                    <button
                      key={facet.value}
                      type="button"
                      onClick={() =>
                        onDecadeChange(decade === value ? null : value)
                      }
                      aria-pressed={decade === value}
                      className={chipClass(decade === value)}
                    >
                      {facet.value}s
                    </button>
                  )
                })}
              </div>
            </div>
          )}

          {genres.length > 0 && (
            <div className="min-w-0">
              <p className="pv-label mb-1">Genre</p>
              <div className="flex flex-wrap gap-1">
                <button
                  type="button"
                  onClick={() => onGenreChange(null)}
                  aria-pressed={genre === null}
                  className={chipClass(genre === null)}
                >
                  All
                </button>
                {genres.map((facet) => (
                  <button
                    key={facet.value}
                    type="button"
                    onClick={() =>
                      onGenreChange(genre === facet.value ? null : facet.value)
                    }
                    aria-pressed={genre === facet.value}
                    className={chipClass(genre === facet.value)}
                  >
                    {facet.value}
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </section>
  )
}
