import { useCallback, useEffect, useState } from 'react'

import { ApiError, api } from '../../lib/api'
import type { AdminSettings as AdminSettingsData } from '../../lib/types'

function MinusIcon() {
  return (
    <svg viewBox="0 0 20 20" className="size-3.5" aria-hidden="true">
      <path d="M5 10h10" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
    </svg>
  )
}

function PlusIcon() {
  return (
    <svg viewBox="0 0 20 20" className="size-3.5" aria-hidden="true">
      <path d="M10 5v10M5 10h10" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
    </svg>
  )
}

export function AdminSettings() {
  const [settings, setSettings] = useState<AdminSettingsData | null>(null)
  const [goal, setGoal] = useState(3)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)

    try {
      const next = await api.getAdminSettings()
      setSettings(next)
      setGoal(next.daily_streak_goal)
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : 'Could not load league settings',
      )
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  function changeGoal(next: number) {
    if (!settings) return
    setNotice(null)
    setGoal(Math.min(settings.max, Math.max(settings.min, next)))
  }

  async function save() {
    if (!settings || saving) return
    setSaving(true)
    setError(null)
    setNotice(null)

    try {
      const next = await api.updateAdminSettings(goal)
      setSettings(next)
      setGoal(next.daily_streak_goal)
      setNotice('Daily streak goal updated for everyone.')
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : 'Could not save league settings',
      )
    } finally {
      setSaving(false)
    }
  }

  if (loading) {
    return (
      <div className="pv-admin-panel">
        <div className="h-5 w-32 animate-pulse rounded-full bg-ink/10" />
        <div className="mt-4 h-24 animate-pulse rounded-xl bg-ink/10" />
      </div>
    )
  }

  if (error && !settings) {
    return (
      <div className="pv-admin-panel text-center">
        <p className="text-sm text-danger">{error}</p>
        <button type="button" onClick={() => void load()} className="pv-admin-button mt-3">
          Try again
        </button>
      </div>
    )
  }

  if (!settings) return null

  const changed = goal !== settings.daily_streak_goal

  return (
    <div className="pv-admin-panel">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <span className="pv-admin-label">League rules</span>
            <span className="pv-sticker pv-sticker-paper">App-wide</span>
          </div>
          <h3 className="mt-1 font-serif text-2xl tracking-[-0.02em]">
            Daily streak goal
          </h3>
          <p className="mt-1 max-w-lg text-sm leading-relaxed text-muted">
            How many picks complete a calendar day for everyone&apos;s streak.
            Changing this recalculates current and longest streaks from existing
            picks.
          </p>
        </div>

        <span className="pv-admin-chip">{settings.daily_streak_goal} picks / day</span>
      </div>

      <div className="mt-5 flex flex-wrap items-end gap-4">
        <div>
          <label
            htmlFor="daily-streak-goal"
            className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted"
          >
            Picks required
          </label>
          <div className="mt-1.5 flex items-center gap-2">
            <button
              type="button"
              onClick={() => changeGoal(goal - 1)}
              disabled={goal <= settings.min || saving}
              aria-label="Decrease daily streak goal"
              className="pv-admin-button size-10 px-0"
            >
              <MinusIcon />
            </button>
            <input
              id="daily-streak-goal"
              type="number"
              min={settings.min}
              max={settings.max}
              step={1}
              value={goal}
              disabled={saving}
              onChange={(event) => changeGoal(Number(event.target.value) || settings.min)}
              className="pv-admin-input w-20 text-center font-serif text-2xl"
            />
            <button
              type="button"
              onClick={() => changeGoal(goal + 1)}
              disabled={goal >= settings.max || saving}
              aria-label="Increase daily streak goal"
              className="pv-admin-button size-10 px-0"
            >
              <PlusIcon />
            </button>
          </div>
          <p className="mt-1.5 font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
            Allowed {settings.min}–{settings.max}
          </p>
        </div>

        <div className="min-w-0 flex-1 rounded-xl border-2 border-line bg-paper px-3 py-2">
          <p className="font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
            Preview
          </p>
          <p className="mt-1 text-sm leading-snug">
            A day counts once a voter makes{' '}
            <span className="font-semibold text-ink">{goal} picks</span>. Their
            bar will show <span className="font-semibold text-ink">x / {goal}</span>{' '}
            today.
          </p>
        </div>

        <button
          type="button"
          onClick={() => void save()}
          disabled={!changed || saving}
          className="pv-admin-button pv-admin-button-accent ml-auto"
        >
          {saving ? 'Saving…' : changed ? 'Save goal' : 'Saved'}
        </button>
      </div>

      {notice && (
        <p role="status" className="mt-3 text-sm text-ok">
          {notice}
        </p>
      )}
      {error && (
        <p role="alert" className="mt-3 text-sm text-danger">
          {error}
        </p>
      )}
    </div>
  )
}
