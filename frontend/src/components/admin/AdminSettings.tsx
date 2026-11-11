import { useCallback, useEffect, useState } from 'react'

import { ApiError, api } from '../../lib/api'
import type { AdminSettings as AdminSettingsData } from '../../lib/types'

type NumericKey =
  | 'daily_streak_goal'
  | 'rank_weight_strength'
  | 'rank_weight_scale'
  | 'rank_weight_min_avg'
  | 'stats_session_gap_minutes'
  | 'stats_fast_decision_seconds'
  | 'stats_upset_threshold_pct'
  | 'stats_side_bias_margin_pct'
  | 'stats_low_agreement_pct'
  | 'stats_min_votes_for_flags'
  | 'stats_target_duels_per_song'

type Draft = {
  rank_weighting: boolean
} & Record<NumericKey, number>

const LIVE_KEYS: NumericKey[] = [
  'rank_weight_strength',
  'rank_weight_scale',
  'rank_weight_min_avg',
]

function draftFrom(settings: AdminSettingsData): Draft {
  return {
    daily_streak_goal: settings.daily_streak_goal,
    rank_weighting: settings.rank_weighting,
    rank_weight_strength: settings.rank_weight_strength,
    rank_weight_scale: settings.rank_weight_scale,
    rank_weight_min_avg: settings.rank_weight_min_avg,
    stats_session_gap_minutes: settings.stats_session_gap_minutes,
    stats_fast_decision_seconds: settings.stats_fast_decision_seconds,
    stats_upset_threshold_pct: settings.stats_upset_threshold_pct,
    stats_side_bias_margin_pct: settings.stats_side_bias_margin_pct,
    stats_low_agreement_pct: settings.stats_low_agreement_pct,
    stats_min_votes_for_flags: settings.stats_min_votes_for_flags,
    stats_target_duels_per_song: settings.stats_target_duels_per_song,
  }
}

function Field({
  id,
  label,
  hint,
  value,
  defaultValue,
  range,
  step,
  onChange,
}: {
  id: string
  label: string
  hint: string
  value: number
  defaultValue: number
  range: [number, number]
  step: number
  onChange: (value: number) => void
}) {
  return (
    <label htmlFor={id} className="block rounded-xl border border-line bg-paper p-3">
      <span className="flex flex-wrap items-baseline justify-between gap-2">
        <span className="text-sm font-medium text-ink">{label}</span>
        <span className="pv-admin-chip">default {defaultValue}</span>
      </span>
      <span className="mt-1 block text-xs leading-relaxed text-muted">{hint}</span>
      <span className="mt-2 flex items-center gap-2">
        <input
          id={id}
          type="number"
          min={range[0]}
          max={range[1]}
          step={step}
          value={value}
          onChange={(event) => onChange(Number(event.target.value))}
          className="pv-admin-input w-28 font-mono text-sm"
        />
        <span className="font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
          {range[0]}–{range[1]}
        </span>
      </span>
    </label>
  )
}

export function AdminSettings() {
  const [settings, setSettings] = useState<AdminSettingsData | null>(null)
  const [draft, setDraft] = useState<Draft | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [confirmOpen, setConfirmOpen] = useState(false)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)

    try {
      const next = await api.getAdminSettings()
      setSettings(next)
      setDraft(draftFrom(next))
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

  function change<K extends keyof Draft>(key: K, value: Draft[K]) {
    setNotice(null)
    setDraft((current) => (current ? { ...current, [key]: value } : current))
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

  if (!settings || !draft) return null

  const changedKeys = (Object.keys(draft) as Array<keyof Draft>).filter(
    (key) => draft[key] !== settings[key],
  )
  const liveChanged =
    draft.rank_weighting !== settings.rank_weighting ||
    LIVE_KEYS.some((key) => draft[key] !== settings[key])
  const changed = changedKeys.length > 0

  function patchFromDraft(): Record<string, number | boolean> {
    const patch: Record<string, number | boolean> = {}
    if (!draft) return patch
    for (const key of changedKeys) {
      patch[key] = draft[key]
    }
    return patch
  }

  async function save() {
    if (!settings || !draft || saving || !changed) return
    setSaving(true)
    setError(null)
    setNotice(null)

    try {
      const next = await api.updateAdminSettings(
        patchFromDraft() as Parameters<typeof api.updateAdminSettings>[0],
      )
      setSettings(next)
      setDraft(draftFrom(next))
      setNotice(
        liveChanged
          ? 'Settings saved. Live duel selection uses the new values immediately; changing parameters mid-vote starts a new catch-up period.'
          : 'Statistics settings saved. Voting and duel selection were not affected.',
      )
      setConfirmOpen(false)
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

  function resetDefaults() {
    if (!settings) return
    setNotice(null)
    setDraft({
      ...draftFrom(settings),
      daily_streak_goal: settings.defaults.daily_streak_goal,
      rank_weighting: true,
      rank_weight_strength: settings.defaults.rank_weight_strength,
      rank_weight_scale: settings.defaults.rank_weight_scale,
      rank_weight_min_avg: settings.defaults.rank_weight_min_avg,
      stats_session_gap_minutes: settings.defaults.stats_session_gap_minutes,
      stats_fast_decision_seconds: settings.defaults.stats_fast_decision_seconds,
      stats_upset_threshold_pct: settings.defaults.stats_upset_threshold_pct,
      stats_side_bias_margin_pct: settings.defaults.stats_side_bias_margin_pct,
      stats_low_agreement_pct: settings.defaults.stats_low_agreement_pct,
      stats_min_votes_for_flags: settings.defaults.stats_min_votes_for_flags,
      stats_target_duels_per_song: settings.defaults.stats_target_duels_per_song,
    })
  }

  return (
    <div className="space-y-4">
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
        <div className="mt-4 max-w-xs">
          <Field
            id="daily-streak-goal"
            label="Picks required"
            hint="A day counts once a voter makes this many picks."
            value={draft.daily_streak_goal}
            defaultValue={settings.defaults.daily_streak_goal}
            range={settings.ranges.daily_streak_goal}
            step={1}
            onChange={(value) => change('daily_streak_goal', value)}
          />
        </div>
      </div>

      <div className="pv-admin-panel">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <p className="pv-admin-label">Live pairing</p>
            <h3 className="mt-1 font-serif text-2xl tracking-[-0.02em]">
              Duel selection
            </h3>
            <p className="mt-1 max-w-2xl text-sm leading-relaxed text-muted">
              These values change which duels are served next. Changing any of
              them while a vote is running starts a new catch-up period, because
              the comparison targets are re-based immediately.
            </p>
          </div>
          <span className="pv-admin-chip">
            {draft.rank_weighting ? 'Weighting on' : 'Weighting off'}
          </span>
        </div>

        <label className="mt-4 flex items-center gap-3 rounded-xl border border-line bg-paper p-3">
          <input
            type="checkbox"
            checked={draft.rank_weighting}
            onChange={(event) => change('rank_weighting', event.target.checked)}
            className="size-4 accent-[var(--color-cobalt)]"
          />
          <span>
            <span className="block text-sm font-medium text-ink">
              Rank weighting
            </span>
            <span className="block text-xs text-muted">
              Give higher-rated songs proportionally more duels so the top of the
              list settles faster.
            </span>
          </span>
        </label>

        <div className="mt-3 grid gap-3 md:grid-cols-3">
          <Field
            id="rank-weight-strength"
            label="Strength"
            hint="How many extra comparisons the #1 song is treated as having."
            value={draft.rank_weight_strength}
            defaultValue={settings.defaults.rank_weight_strength}
            range={settings.ranges.rank_weight_strength}
            step={0.5}
            onChange={(value) => change('rank_weight_strength', value)}
          />
          <Field
            id="rank-weight-scale"
            label="Scale (tracks)"
            hint="By this rank the extra weight has decayed to about 1/e of the strength."
            value={draft.rank_weight_scale}
            defaultValue={settings.defaults.rank_weight_scale}
            range={settings.ranges.rank_weight_scale}
            step={25}
            onChange={(value) => change('rank_weight_scale', value)}
          />
          <Field
            id="rank-weight-min-avg"
            label="Minimum average comparisons"
            hint="Weighting only starts once the playlist averages this many comparisons per song."
            value={draft.rank_weight_min_avg}
            defaultValue={settings.defaults.rank_weight_min_avg}
            range={settings.ranges.rank_weight_min_avg}
            step={0.5}
            onChange={(value) => change('rank_weight_min_avg', value)}
          />
        </div>
      </div>

      <div className="pv-admin-panel">
        <div>
          <p className="pv-admin-label">Statistics</p>
          <h3 className="mt-1 font-serif text-2xl tracking-[-0.02em]">
            Definitions and flag thresholds
          </h3>
          <p className="mt-1 max-w-2xl text-sm leading-relaxed text-muted">
            These only change how the admin statistics are calculated. They never
            change what a voter sees, how a vote is recorded or which duel is
            served.
          </p>
        </div>

        <div className="mt-4 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          <Field
            id="stats-session-gap-minutes"
            label="Session gap (minutes)"
            hint="A gap longer than this starts a new voting session and marks the decision as idle."
            value={draft.stats_session_gap_minutes}
            defaultValue={settings.defaults.stats_session_gap_minutes}
            range={settings.ranges.stats_session_gap_minutes}
            step={1}
            onChange={(value) => change('stats_session_gap_minutes', value)}
          />
          <Field
            id="stats-fast-decision-seconds"
            label="Fast decision (seconds)"
            hint="A median decision time below this flags a voter as a fast clicker."
            value={draft.stats_fast_decision_seconds}
            defaultValue={settings.defaults.stats_fast_decision_seconds}
            range={settings.ranges.stats_fast_decision_seconds}
            step={0.1}
            onChange={(value) => change('stats_fast_decision_seconds', value)}
          />
          <Field
            id="stats-upset-threshold-pct"
            label="Upset threshold (%)"
            hint="A winner below this pre-vote win chance counts as an upset."
            value={draft.stats_upset_threshold_pct}
            defaultValue={settings.defaults.stats_upset_threshold_pct}
            range={settings.ranges.stats_upset_threshold_pct}
            step={1}
            onChange={(value) => change('stats_upset_threshold_pct', value)}
          />
          <Field
            id="stats-side-bias-margin-pct"
            label="Side-bias margin (%)"
            hint="A voter is side-biased when their share of left picks is outside 50% by more than this."
            value={draft.stats_side_bias_margin_pct}
            defaultValue={settings.defaults.stats_side_bias_margin_pct}
            range={settings.ranges.stats_side_bias_margin_pct}
            step={1}
            onChange={(value) => change('stats_side_bias_margin_pct', value)}
          />
          <Field
            id="stats-low-agreement-pct"
            label="Low agreement (%)"
            hint="A voter is flagged when they agree with the current standings less often than this."
            value={draft.stats_low_agreement_pct}
            defaultValue={settings.defaults.stats_low_agreement_pct}
            range={settings.ranges.stats_low_agreement_pct}
            step={1}
            onChange={(value) => change('stats_low_agreement_pct', value)}
          />
          <Field
            id="stats-min-votes-for-flags"
            label="Minimum votes for flags"
            hint="Low-agreement flags need at least this many votes. Fast and side-bias flags are rate based."
            value={draft.stats_min_votes_for_flags}
            defaultValue={settings.defaults.stats_min_votes_for_flags}
            range={settings.ranges.stats_min_votes_for_flags}
            step={1}
            onChange={(value) => change('stats_min_votes_for_flags', value)}
          />
          <Field
            id="stats-target-duels-per-song"
            label="Target duels per song"
            hint="The progress bar and forecast compare the playlist against this target."
            value={draft.stats_target_duels_per_song}
            defaultValue={settings.defaults.stats_target_duels_per_song}
            range={settings.ranges.stats_target_duels_per_song}
            step={1}
            onChange={(value) => change('stats_target_duels_per_song', value)}
          />
        </div>
      </div>

      <div className="pv-admin-panel flex flex-wrap items-center gap-3">
        <button
          type="button"
          onClick={resetDefaults}
          className="pv-admin-button"
        >
          Reset to defaults
        </button>
        <button
          type="button"
          onClick={() => (liveChanged ? setConfirmOpen(true) : void save())}
          disabled={!changed || saving}
          className="pv-admin-button pv-admin-button-accent ml-auto"
        >
          {saving ? 'Saving…' : changed ? 'Save settings' : 'Saved'}
        </button>
        <p className="w-full font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
          {changedKeys.length === 0
            ? 'No unsaved changes.'
            : `${changedKeys.length} unsaved change${changedKeys.length === 1 ? '' : 's'}${liveChanged ? ' · live pairing included' : ' · statistics only'}`}
        </p>
      </div>

      {notice && (
        <p role="status" className="text-sm text-ok">
          {notice}
        </p>
      )}
      {error && (
        <p role="alert" className="text-sm text-danger">
          {error}
        </p>
      )}

      {confirmOpen && (
        <div className="fixed inset-0 z-50 grid place-items-center bg-ink/40 p-4">
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="pairing-confirm-title"
            className="w-full max-w-lg rounded-2xl border-2 border-ink bg-card p-5 shadow-[6px_6px_0_0_var(--color-ink)]"
          >
            <p className="pv-admin-label">Live duel selection</p>
            <h3
              id="pairing-confirm-title"
              className="mt-1 font-serif text-2xl tracking-[-0.02em]"
            >
              Change the live pairing?
            </h3>
            <p className="mt-2 text-sm leading-relaxed text-muted">
              This changes which duels are served to voters right now. The vote is
              running, and changing the strength, scale or minimum average
              mid-vote starts a new catch-up period: the algorithm re-bases every
              song&apos;s comparison target, so the settling forecast will jump.
            </p>
            <p className="mt-2 text-sm leading-relaxed text-muted">
              Votes already cast and ratings already earned are not changed.
            </p>
            <div className="mt-4 flex flex-wrap justify-end gap-2">
              <button
                type="button"
                onClick={() => setConfirmOpen(false)}
                className="pv-admin-button"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => void save()}
                disabled={saving}
                className="pv-admin-button pv-admin-button-accent"
              >
                {saving ? 'Saving…' : 'Change live pairing'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
