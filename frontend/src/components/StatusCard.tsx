import { useEffect, useState } from 'react'

type Health = {
  status: string
  version: string
  environment: string
  uptime_seconds: number
}

type Readiness = {
  status: string
  database: string
}

type Status = 'checking' | 'healthy' | 'degraded' | 'offline'

const statusStyles: Record<Status, { dot: string; label: string }> = {
  checking: { dot: 'bg-muted', label: 'Checking' },
  healthy: { dot: 'bg-ok', label: 'Operational' },
  degraded: { dot: 'bg-accent', label: 'Database down' },
  offline: { dot: 'bg-accent', label: 'Offline' },
}

function formatUptime(seconds: number): string {
  if (seconds < 60) return `${seconds}s`
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes}m`
  const hours = Math.floor(minutes / 60)
  return `${hours}h ${minutes % 60}m`
}

function statusOf(health: Health | null, readiness: Readiness | null): Status {
  if (!health) return 'offline'
  if (readiness?.database !== 'up') return 'degraded'
  return 'healthy'
}

export function StatusCard() {
  const [health, setHealth] = useState<Health | null>(null)
  const [readiness, setReadiness] = useState<Readiness | null>(null)
  const [checked, setChecked] = useState(false)

  useEffect(() => {
    let active = true

    async function refresh() {
      const [healthResult, readinessResult] = await Promise.allSettled([
        fetch('/healthz').then((response) => {
          if (!response.ok) throw new Error(`healthz ${response.status}`)
          return response.json() as Promise<Health>
        }),
        fetch('/readyz').then((response) => response.json() as Promise<Readiness>),
      ])

      if (!active) return

      setHealth(healthResult.status === 'fulfilled' ? healthResult.value : null)
      setReadiness(
        readinessResult.status === 'fulfilled' ? readinessResult.value : null,
      )
      setChecked(true)
    }

    void refresh()
    const timer = window.setInterval(() => void refresh(), 10_000)

    return () => {
      active = false
      window.clearInterval(timer)
    }
  }, [])

  const status = checked ? statusOf(health, readiness) : 'checking'
  const styles = statusStyles[status]

  return (
    <section className="panel">
      <div className="flex items-center justify-between border-b border-line px-4 py-2.5">
        <span className="label">System</span>
        <span
          role="status"
          aria-live="polite"
          className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.16em]"
        >
          <span
            className={`size-1.5 ${styles.dot} ${status === 'checking' ? 'animate-pulse' : ''}`}
          />
          {styles.label}
        </span>
      </div>

      <dl className="grid grid-cols-2">
        <div className="border-b border-r border-line px-4 py-3">
          <dt className="label">Backend</dt>
          <dd className="mt-1 font-mono text-xs">{health ? `v${health.version}` : '—'}</dd>
        </div>
        <div className="border-b border-line px-4 py-3">
          <dt className="label">Environment</dt>
          <dd className="mt-1 font-mono text-xs">{health?.environment ?? '—'}</dd>
        </div>
        <div className="border-r border-line px-4 py-3">
          <dt className="label">Uptime</dt>
          <dd className="mt-1 font-mono text-xs">
            {health ? formatUptime(health.uptime_seconds) : '—'}
          </dd>
        </div>
        <div className="px-4 py-3">
          <dt className="label">Postgres</dt>
          <dd className="mt-1 font-mono text-xs">{readiness?.database ?? '—'}</dd>
        </div>
      </dl>
    </section>
  )
}
