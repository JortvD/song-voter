type MixtapeCaseProps = {
  title: string
  coverUrl: string | null
  trackCount: number
  description?: string | null
  revealed?: boolean
  className?: string
}

export function MixtapeCase({
  title,
  coverUrl,
  trackCount,
  description,
  revealed = false,
  className = '',
}: MixtapeCaseProps) {
  return (
    <div className={`pv-mixtape-case ${className}`}>
      <div className="pv-mixtape-top">
        <span>Side A</span>
        <span>{trackCount > 0 ? `${trackCount} songs` : 'Blank tape'}</span>
      </div>

      <div className="pv-mixtape-cover">
        {coverUrl ? (
          <img src={coverUrl} alt={`${title} cover`} />
        ) : (
          <div className="grid h-full place-items-center bg-[linear-gradient(135deg,var(--playlist-accent-tint),transparent_60%)]">
            <span className="text-center">
              <span className="block font-serif text-6xl leading-none text-muted/30">
                ♪
              </span>
              <span className="mt-2 block font-mono text-[9px] uppercase tracking-[0.18em] text-muted">
                No cover yet
              </span>
            </span>
          </div>
        )}

        <span className="pv-sticker pv-sticker-paper pv-mixtape-sticker">
          {trackCount === 0 ? 'Blank tape' : revealed ? 'Standings live' : 'Sealed'}
        </span>
      </div>

      <h2 className="pv-mixtape-title">{title}</h2>

      {description ? (
        <p className="mt-2 line-clamp-2 text-sm leading-relaxed text-muted">
          {description}
        </p>
      ) : (
        <p className="mt-2 line-clamp-2 text-sm leading-relaxed text-muted">
          A shared playlist, a stack of duels, and one group&apos;s taste on the
          line.
        </p>
      )}

      <div className="pv-mixtape-bottom">
        <span>Playlist Voter</span>
        <span>Two songs. One pick.</span>
      </div>
    </div>
  )
}
