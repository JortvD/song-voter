export function StreakFlame({ className = 'size-4' }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" className={className} aria-hidden="true">
      <path
        d="M13.2 2.4c.5 3.7-1.4 5.1-2.7 6.7-1.1 1.4-2 2.8-2 4.6a5.5 5.5 0 0 0 11 0c0-2.2-1.1-4-3-6 .1 1.8-.5 3.2-1.5 4.1.2-2.5-.6-6.9-1.8-9.4Z"
        fill="currentColor"
      />
      <path
        d="M12 12.3c.4 1.4-.2 2.4-.9 3.2-.5.6-.8 1.1-.8 1.8a2.4 2.4 0 0 0 4.8 0c0-1-.5-1.8-1.2-2.6-.1.7-.3 1.2-.7 1.6.1-1.2-.4-2.7-1.2-4Z"
        fill="var(--color-paper)"
        opacity=".9"
      />
    </svg>
  )
}
