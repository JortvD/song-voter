import { useState } from 'react'

import type {
  StoryCard,
  StoryLeaderboard,
  StoryLeaderboardRow,
  StoryTrack,
} from '../lib/types'

const CATEGORY_LABELS: Record<string, string> = {
  all: 'All',
  album: 'Albums',
  artist: 'Artists',
  decade: 'Decades',
  duel: 'Duels',
  tape: 'Tape',
  voter: 'Voters',
}

function categorySticker(category: string): string {
  if (category === 'artist') return 'pv-sticker pv-sticker-cobalt'
  if (category === 'decade') return 'pv-sticker pv-sticker-butter'
  if (category === 'tape') return 'pv-sticker pv-sticker-paper'
  return 'pv-sticker'
}

function StoryCover({
  image,
  title,
  size = 'size-9',
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
      className={`${size} grid shrink-0 place-items-center rounded-lg border-2 border-ink bg-paper font-serif text-sm text-muted/60`}
    >
      ♪
    </span>
  )
}

function TrackRow({ track }: { track: StoryTrack }) {
  return (
    <div className="flex items-center gap-2">
      <StoryCover image={track.album_image_url} title={track.title} size="size-8" />
      <div className="min-w-0 flex-1">
        <p className="truncate text-xs font-medium text-ink">{track.title}</p>
        <p className="truncate font-mono text-[8px] uppercase tracking-[0.1em] text-muted">
          {track.artists}
        </p>
      </div>
      <div className="shrink-0 text-right">
        <p className="max-w-16 truncate font-mono text-[7px] uppercase tracking-[0.12em] text-muted">
          {track.role}
        </p>
        <p className="font-serif text-sm leading-none text-ink">
          {Math.round(track.rating)}
        </p>
      </div>
    </div>
  )
}

export function StoryCardView({ card }: { card: StoryCard }) {
  return (
    <article className="pv-card flex h-full min-w-0 flex-col p-4">
      <div className="flex min-w-0 items-start justify-between gap-2">
        <span className={`${categorySticker(card.category)} max-w-full`}>
          {card.eyebrow}
        </span>
        <span className="shrink-0 font-mono text-[8px] uppercase tracking-[0.14em] text-muted">
          {CATEGORY_LABELS[card.category] ?? card.category}
        </span>
      </div>

      <div className="mt-3 flex min-w-0 items-center gap-3">
        <StoryCover image={card.image_url} title={card.title} size="size-14" />
        <div className="min-w-0">
          <p className="line-clamp-2 break-words font-serif text-lg leading-tight tracking-[-0.02em] [overflow-wrap:anywhere]">
            {card.title}
          </p>
          {card.subtitle && (
            <p className="mt-0.5 truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
              {card.subtitle}
            </p>
          )}
        </div>
      </div>

      <p className="mt-3 break-words text-sm leading-relaxed text-muted [overflow-wrap:anywhere]">
        {card.body}
      </p>

      {card.stats.length > 0 && (
        <div className="mt-3 grid grid-cols-2 gap-2 border-t-2 border-ink/10 pt-2 sm:grid-cols-3">
          {card.stats.map((stat) => (
            <div key={stat.label} className="min-w-0">
              <p className="truncate font-mono text-[7px] uppercase tracking-[0.14em] text-muted">
                {stat.label}
              </p>
              <p className="mt-0.5 break-words font-serif text-lg leading-none [overflow-wrap:anywhere]">
                {stat.value}
              </p>
            </div>
          ))}
        </div>
      )}

      {card.tracks.length > 0 && (
        <div className="mt-3 space-y-2 border-t-2 border-ink/10 pt-2">
          {card.tracks.map((track) => (
            <TrackRow key={`${card.id}-${track.role}-${track.id}`} track={track} />
          ))}
        </div>
      )}
    </article>
  )
}

export function StoryCardGrid({ cards }: { cards: StoryCard[] }) {
  const [category, setCategory] = useState('all')

  if (cards.length === 0) return null

  const categories = ['all', ...new Set(cards.map((card) => card.category))]
  const visible = category === 'all' ? cards : cards.filter((card) => card.category === category)

  return (
    <div className="mt-3">
      <div className="flex flex-wrap gap-1.5">
        {categories.map((option) => (
          <button
            key={option}
            type="button"
            onClick={() => setCategory(option)}
            aria-pressed={category === option}
            className={`rounded-full border-2 px-2.5 py-1 font-mono text-[8px] uppercase tracking-[0.12em] transition ${
              category === option
                ? 'border-ink bg-ink text-paper'
                : 'border-line bg-paper text-muted hover:border-ink hover:text-ink'
            }`}
          >
            {CATEGORY_LABELS[option] ?? option}
          </button>
        ))}
      </div>

      <div className="mt-3 grid gap-4 md:grid-cols-2 xl:grid-cols-3">
        {visible.map((card) => (
          <StoryCardView key={card.id} card={card} />
        ))}
      </div>
    </div>
  )
}

function LeaderboardRow({ row }: { row: StoryLeaderboardRow }) {
  return (
    <li className="flex items-center gap-2 py-2">
      <span className="w-5 shrink-0 font-serif text-lg leading-none text-muted">
        {String(row.rank).padStart(2, '0')}
      </span>
      <StoryCover image={row.image_url} title={row.title} size="size-8" />
      <div className="min-w-0 flex-1">
        <p className="truncate text-xs font-medium text-ink">{row.title}</p>
        {row.subtitle && (
          <p className="truncate font-mono text-[8px] uppercase tracking-[0.1em] text-muted">
            {row.subtitle}
          </p>
        )}
      </div>
      <div className="shrink-0 text-right">
        <p className="font-serif text-sm leading-none text-ink">{row.value}</p>
        <p className="max-w-20 truncate font-mono text-[8px] uppercase tracking-[0.1em] text-muted">
          {row.detail}
        </p>
      </div>
    </li>
  )
}

export function StoryLeaderboards({
  leaderboards,
}: {
  leaderboards: StoryLeaderboard[]
}) {
  const categories = ['album', 'artist', 'decade', 'duel', 'voter'].filter((category) =>
    leaderboards.some((board) => board.category === category),
  )
  const [selected, setSelected] = useState('')
  const active = categories.includes(selected) ? selected : (categories[0] ?? '')
  const visible = leaderboards.filter((board) => board.category === active)

  if (leaderboards.length === 0) return null

  return (
    <div className="mt-3">
      <div className="flex flex-wrap gap-1.5">
        {categories.map((option) => (
          <button
            key={option}
            type="button"
            onClick={() => setSelected(option)}
            aria-pressed={active === option}
            className={`rounded-full border-2 px-2.5 py-1 font-mono text-[8px] uppercase tracking-[0.12em] transition ${
              active === option
                ? 'border-ink bg-ink text-paper'
                : 'border-line bg-paper text-muted hover:border-ink hover:text-ink'
            }`}
          >
            {CATEGORY_LABELS[option] ?? option}
          </button>
        ))}
      </div>

      <div className="mt-3 grid gap-4 md:grid-cols-2">
        {visible.map((board) => (
          <article key={board.id} className="min-w-0 rounded-xl border border-line bg-paper p-3">
            <div className="flex min-w-0 items-end justify-between gap-2 border-b border-line pb-2">
              <h4 className="min-w-0 break-words font-serif text-lg leading-tight text-ink">
                {board.title}
              </h4>
              <span className="shrink-0 font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
                Top {board.rows.length}
              </span>
            </div>
            <ol className="divide-y divide-line">
              {board.rows.map((row) => (
                <LeaderboardRow key={`${board.id}-${row.rank}`} row={row} />
              ))}
            </ol>
            <p className="mt-1 font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
              {board.metric_label} · {board.detail_label}
            </p>
          </article>
        ))}
      </div>
    </div>
  )
}
