type BrandMarkProps = {
  className?: string
}

/**
 * Two overlapping tape cards with an A/B split. The orange card is the "pick";
 * the cream card is the alternative.
 */
export function BrandMark({ className = 'size-9' }: BrandMarkProps) {
  return (
    <svg
      viewBox="0 0 44 44"
      className={className}
      role="img"
      aria-label="Playlist Voter"
    >
      <rect
        x="4"
        y="8"
        width="23"
        height="28"
        rx="5"
        transform="rotate(-7 15.5 22)"
        fill="var(--color-paper)"
        stroke="var(--color-ink)"
        strokeWidth="2"
      />
      <rect
        x="17"
        y="8"
        width="23"
        height="28"
        rx="5"
        transform="rotate(7 28.5 22)"
        fill="var(--playlist-accent)"
        stroke="var(--color-ink)"
        strokeWidth="2"
      />
      <text
        x="12.5"
        y="27"
        textAnchor="middle"
        fontFamily="'Bricolage Grotesque', 'DM Sans', sans-serif"
        fontSize="14"
        fontWeight="700"
        fill="var(--color-ink)"
      >
        A
      </text>
      <text
        x="31.5"
        y="27"
        textAnchor="middle"
        fontFamily="'Bricolage Grotesque', 'DM Sans', sans-serif"
        fontSize="14"
        fontWeight="700"
        fill="var(--playlist-accent-ink)"
      >
        B
      </text>
    </svg>
  )
}
