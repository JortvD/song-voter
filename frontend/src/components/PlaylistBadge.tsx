import type { ChartStatus } from '../lib/types'

type PlaylistBadgeProps = {
  status: ChartStatus | null
  loading?: boolean
  compact?: boolean
}

export function PlaylistBadge({ status, loading = false, compact = false }: PlaylistBadgeProps) {
  if (loading) {
    return (
      <div className="flex min-w-0 items-center gap-2.5">
        <span className="size-9 animate-pulse rounded-lg border-2 border-line bg-card" />
        <span className="min-w-0 flex-1">
          <span className="block h-2 w-16 animate-pulse rounded bg-line/70" />
          <span className="mt-1 block h-3 w-28 animate-pulse rounded bg-line/70" />
        </span>
      </div>
    )
  }

  const name = status?.playlist_name ?? 'No tape yet'
  const cover = status?.playlist_image_url

  return (
    <div className="flex min-w-0 items-center gap-2.5">
      <span className="pv-cover-thumb" aria-hidden="true">
        {cover ? (
          <img src={cover} alt="" loading="lazy" />
        ) : (
          <span className="font-serif text-base leading-none text-muted/50">♪</span>
        )}
      </span>

      <span className="min-w-0">
        <span className="flex items-center gap-1.5">
          <span className="pv-sticker px-1.5 py-0.5 text-[9px]">
            {status?.playlist_name ? 'Now voting' : 'Blank tape'}
          </span>
          {!compact && status && (
            <span className="hidden font-mono text-[9px] uppercase tracking-[0.16em] text-muted sm:inline">
              Side A
            </span>
          )}
        </span>
        <span className="mt-0.5 block truncate font-serif text-sm leading-tight tracking-[-0.01em]">
          {name}
        </span>
      </span>
    </div>
  )
}
