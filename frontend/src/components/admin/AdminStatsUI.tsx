import type { ReactNode } from 'react'

import type { AdminHeatmap } from '../../lib/types'

export function formatNumber(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return '—'
  return Math.round(value).toLocaleString()
}

export function formatDecimal(value: number | null | undefined, digits = 1) {
  if (value == null || !Number.isFinite(value)) return '—'
  return value.toLocaleString(undefined, {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  })
}

export function formatPercent(value: number | null | undefined, digits = 0) {
  if (value == null || !Number.isFinite(value)) return '—'
  return `${value.toFixed(digits)}%`
}

export function formatSeconds(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return '—'
  if (value < 10) return `${value.toFixed(1)}s`
  if (value < 60) return `${Math.round(value)}s`
  return `${Math.round(value / 60)}m ${Math.round(value % 60)}s`
}

export function formatHours(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return '—'
  if (value < 1) return `${Math.round(value * 60)} min`
  if (value < 48) return `${value.toFixed(1)} h`
  if (value < 24 * 365) return `${(value / 24).toFixed(1)} d`
  return `${(value / (24 * 365)).toFixed(1)} y`
}

export function formatWhen(value: string | null | undefined) {
  if (!value) return '—'
  return new Date(value).toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

export function formatDay(value: string | null | undefined) {
  if (!value) return '—'
  return new Date(value).toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  })
}

export function AsOf({ value }: { value: string | null | undefined }) {
  if (!value) return null
  return (
    <span className="font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
      as of{' '}
      {new Date(value).toLocaleTimeString(undefined, {
        hour: '2-digit',
        minute: '2-digit',
      })}
    </span>
  )
}

export function StatTile({
  label,
  value,
  hint,
  tone,
}: {
  label: string
  value: string
  hint?: string
  tone?: 'ok' | 'danger' | 'accent'
}) {
  const toneClass =
    tone === 'ok'
      ? 'text-ok'
      : tone === 'danger'
        ? 'text-danger'
        : tone === 'accent'
          ? 'text-ink'
          : 'text-ink'
  return (
    <div className="rounded-xl border border-line bg-paper p-3">
      <p className="pv-admin-label text-[8px]">{label}</p>
      <p className={`mt-1 font-serif text-2xl leading-none ${toneClass}`}>{value}</p>
      {hint && <p className="mt-1 text-[11px] leading-snug text-muted">{hint}</p>}
    </div>
  )
}

export function Explainer({ children }: { children: ReactNode }) {
  return (
    <p className="mt-1 text-[11px] leading-relaxed text-muted">{children}</p>
  )
}

export function TrendLine({
  points,
  height = 120,
  color = 'var(--color-cobalt)',
  fill = true,
  formatValue,
}: {
  points: Array<{ x: number; y: number; label?: string }>
  height?: number
  color?: string
  fill?: boolean
  formatValue?: (value: number) => string
}) {
  if (points.length === 0) {
    return <p className="text-sm text-muted">Not enough data yet.</p>
  }
  const width = 600
  const padding = { top: 8, right: 8, bottom: 18, left: 34 }
  const innerWidth = width - padding.left - padding.right
  const innerHeight = height - padding.top - padding.bottom
  const xs = points.map((point) => point.x)
  const ys = points.map((point) => point.y)
  const minX = Math.min(...xs)
  const maxX = Math.max(...xs)
  const minY = Math.min(...ys)
  const maxY = Math.max(...ys)
  const spanX = maxX - minX || 1
  const spanY = maxY - minY || 1
  const sx = (x: number) => padding.left + ((x - minX) / spanX) * innerWidth
  const sy = (y: number) => padding.top + innerHeight - ((y - minY) / spanY) * innerHeight
  const path = points
    .map((point, index) => `${index === 0 ? 'M' : 'L'} ${sx(point.x).toFixed(2)} ${sy(point.y).toFixed(2)}`)
    .join(' ')
  const area = `${path} L ${sx(maxX).toFixed(2)} ${(padding.top + innerHeight).toFixed(2)} L ${sx(minX).toFixed(2)} ${(padding.top + innerHeight).toFixed(2)} Z`

  return (
    <div className="w-full">
      <svg
        viewBox={`0 0 ${width} ${height}`}
        className="w-full"
        role="img"
        aria-label="Trend chart"
      >
        <line
          x1={padding.left}
          x2={width - padding.right}
          y1={padding.top + innerHeight}
          y2={padding.top + innerHeight}
          stroke="var(--color-line)"
          strokeWidth="1"
        />
        <line
          x1={padding.left}
          x2={padding.left}
          y1={padding.top}
          y2={padding.top + innerHeight}
          stroke="var(--color-line)"
          strokeWidth="1"
        />
        {fill && <path d={area} fill={color} opacity="0.08" />}
        <path d={path} fill="none" stroke={color} strokeWidth="2" strokeLinejoin="round" />
        <text
          x={padding.left - 4}
          y={padding.top + 4}
          textAnchor="end"
          className="fill-muted font-mono text-[8px]"
        >
          {formatValue ? formatValue(maxY) : maxY.toFixed(0)}
        </text>
        <text
          x={padding.left - 4}
          y={padding.top + innerHeight}
          textAnchor="end"
          className="fill-muted font-mono text-[8px]"
        >
          {formatValue ? formatValue(minY) : minY.toFixed(0)}
        </text>
        {points.length <= 30 &&
          points.map((point, index) => (
            <circle
              key={index}
              cx={sx(point.x)}
              cy={sy(point.y)}
              r="2"
              fill={color}
              opacity="0.75"
            />
          ))}
        {points.length > 1 && (
          <>
            <text
              x={padding.left}
              y={height - 4}
              className="fill-muted font-mono text-[8px]"
            >
              {points[0].label ?? ''}
            </text>
            <text
              x={width - padding.right}
              y={height - 4}
              textAnchor="end"
              className="fill-muted font-mono text-[8px]"
            >
              {points[points.length - 1].label ?? ''}
            </text>
          </>
        )}
      </svg>
    </div>
  )
}

export function BarList({
  data,
  color = 'var(--color-cobalt)',
  valueLabel,
}: {
  data: Array<{ label: string; value: number; hint?: string }>
  color?: string
  valueLabel?: (value: number) => string
}) {
  const max = Math.max(1, ...data.map((row) => row.value))
  return (
    <div className="space-y-2">
      {data.map((row) => (
        <div key={row.label} title={row.hint}>
          <div className="flex items-center justify-between gap-2 font-mono text-[9px] uppercase tracking-[0.1em] text-muted">
            <span className="truncate">{row.label}</span>
            <span className="shrink-0 text-ink">
              {valueLabel ? valueLabel(row.value) : formatNumber(row.value)}
            </span>
          </div>
          <div className="mt-1 h-2 overflow-hidden rounded-full bg-line/60">
            <div
              className="h-full rounded-full"
              style={{
                width: `${Math.max(1, (row.value / max) * 100)}%`,
                background: color,
              }}
            />
          </div>
        </div>
      ))}
    </div>
  )
}

const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']

export function HeatmapGrid({
  heatmap,
  label,
}: {
  heatmap: AdminHeatmap
  label?: string
}) {
  const max = Math.max(1, heatmap.max)
  return (
    <div>
      <div className="flex gap-1">
        <div className="flex w-8 shrink-0 flex-col justify-around pt-0.5 font-mono text-[8px] uppercase text-muted">
          {WEEKDAYS.map((day) => (
            <span key={day}>{day}</span>
          ))}
        </div>
        <div className="min-w-0 flex-1">
          <div className="grid grid-cols-[repeat(24,minmax(0,1fr))] gap-0.5">
            {heatmap.cells.map((row, dayIndex) =>
              row.map((count, hour) => (
                <span
                  key={`${dayIndex}-${hour}`}
                  title={`${WEEKDAYS[dayIndex]} ${String(hour).padStart(2, '0')}:00 — ${count} ${label ?? 'events'}`}
                  className="aspect-square rounded-[2px] border border-line/40"
                  style={{
                    background:
                      count === 0
                        ? 'transparent'
                        : `color-mix(in srgb, var(--color-cobalt) ${Math.max(10, Math.round((count / max) * 100))}%, var(--color-paper))`,
                  }}
                />
              )),
            )}
          </div>
          <div className="mt-1 flex justify-between font-mono text-[8px] uppercase text-muted">
            <span>00</span>
            <span>06</span>
            <span>12</span>
            <span>18</span>
            <span>23</span>
          </div>
        </div>
      </div>
      <p className="mt-1 font-mono text-[8px] uppercase tracking-[0.1em] text-muted">
        Local time · busiest cell {formatNumber(max)}
      </p>
    </div>
  )
}

export function SectionNote({ children }: { children: ReactNode }) {
  return (
    <p className="rounded-xl border border-line bg-paper px-3 py-2 text-xs leading-relaxed text-muted">
      {children}
    </p>
  )
}
