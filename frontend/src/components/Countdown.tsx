import { useEffect, useState } from 'react'

export function Countdown({
  target,
  label,
}: {
  target: string | null
  label: string
}) {
  const [now, setNow] = useState(() => Date.now())

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000)
    return () => window.clearInterval(timer)
  }, [])

  if (!target) return null
  const targetMs = new Date(target).getTime()
  if (!Number.isFinite(targetMs)) return null

  const remaining = targetMs - now
  let value = 'any moment now'
  if (remaining > 0) {
    const days = Math.floor(remaining / 86_400_000)
    const hours = Math.floor((remaining % 86_400_000) / 3_600_000)
    const minutes = Math.floor((remaining % 3_600_000) / 60_000)
    const seconds = Math.floor((remaining % 60_000) / 1000)

    if (days > 0) value = `${days}d ${hours}h`
    else if (hours > 0) value = `${hours}h ${minutes}m`
    else value = `${minutes}m ${seconds}s`
  }

  return (
    <div className="pv-countdown">
      <span className="pv-countdown-dot" aria-hidden="true" />
      <span className="pv-countdown-label">{label}</span>
      <span className="pv-countdown-value">{value}</span>
    </div>
  )
}
