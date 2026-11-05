import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'

import { ApiError, api } from '../lib/api'
import type { BalancedSettings } from '../lib/types'

type Draft = {
  influence_exponent: string
  confidence_prior: string
  neutral_prior: string
  min_picks: string
}

const DEFAULT_DRAFT: Draft = {
  influence_exponent: '0.5',
  confidence_prior: '10',
  neutral_prior: '20',
  min_picks: '0',
}

function toDraft(settings: BalancedSettings): Draft {
  return {
    influence_exponent: String(settings.influence_exponent),
    confidence_prior: String(settings.confidence_prior),
    neutral_prior: String(settings.neutral_prior),
    min_picks: String(settings.min_picks),
  }
}

export function AdminBalancedSettings() {
  const [settings, setSettings] = useState<BalancedSettings | null>(null)
  const [draft, setDraft] = useState<Draft>(DEFAULT_DRAFT)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [message, setMessage] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    setLoading(true)
    setError(null)

    api
      .getBalancedSettings()
      .then((next) => {
        if (!active) return
        setSettings(next)
        setDraft(toDraft(next))
      })
      .catch((caught: unknown) => {
        if (active) {
          setError(
            caught instanceof ApiError
              ? caught.message
              : 'Could not load the balanced settings',
          )
        }
      })
      .finally(() => {
        if (active) setLoading(false)
      })

    return () => {
      active = false
    }
  }, [])

  function updateField(field: keyof Draft, value: string) {
    setMessage(null)
    setDraft((current) => ({ ...current, [field]: value }))
  }

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setSaving(true)
    setError(null)
    setMessage(null)

    if (Object.values(draft).some((value) => value.trim() === '')) {
      setError('All settings are required.')
      setSaving(false)
      return
    }

    const input = {
      influence_exponent: Number(draft.influence_exponent),
      confidence_prior: Number(draft.confidence_prior),
      neutral_prior: Number(draft.neutral_prior),
      min_picks: Number(draft.min_picks),
    }

    if (Object.values(input).some((value) => !Number.isFinite(value))) {
      setError('All settings must be numbers.')
      setSaving(false)
      return
    }

    if (!Number.isInteger(input.min_picks)) {
      setError('Minimum picks must be a whole number.')
      setSaving(false)
      return
    }

    try {
      const next = await api.updateBalancedSettings(input)
      setSettings(next)
      setDraft(toDraft(next))
      setMessage('Balanced settings saved. The next standings request uses them.')
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : 'Could not save the balanced settings',
      )
    } finally {
      setSaving(false)
    }
  }

  return (
    <section className="panel p-5">
      <div className="flex flex-wrap items-end justify-between gap-3 border-b border-line pb-2">
        <div>
          <h2 className="label">Balanced ranking</h2>
          <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            Tuning for /standings · Balanced
          </p>
        </div>
        {settings && (
          <p className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            Updated {new Date(settings.updated_at).toLocaleString()}
          </p>
        )}
      </div>

      <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted">
        Influence grows with the square root of each voter&apos;s pick count by
        default. Personal scores are shrunk toward neutral until a voter has
        compared a song enough times, and songs with little evidence are pulled
        toward 1000.
      </p>

      {error && (
        <p className="mt-4 border-l-2 border-accent bg-accent/5 px-3 py-2 text-xs text-accent">
          {error}
        </p>
      )}

      <form onSubmit={save} className="mt-4 grid gap-4 sm:grid-cols-2">
        <label className="block">
          <span className="label">Influence exponent</span>
          <input
            type="number"
            step="0.05"
            min="0"
            max="2"
            value={draft.influence_exponent}
            onChange={(event) => updateField('influence_exponent', event.target.value)}
            className="field mt-1"
            disabled={loading || saving}
          />
          <span className="mt-1 block font-mono text-[8px] uppercase tracking-[0.12em] text-muted/70">
            0 = equal · 0.5 = square root · 1 = linear
          </span>
        </label>

        <label className="block">
          <span className="label">Confidence prior</span>
          <input
            type="number"
            step="1"
            min="0"
            max="500"
            value={draft.confidence_prior}
            onChange={(event) => updateField('confidence_prior', event.target.value)}
            className="field mt-1"
            disabled={loading || saving}
          />
          <span className="mt-1 block font-mono text-[8px] uppercase tracking-[0.12em] text-muted/70">
            Higher shrinks voters with few comparisons per song
          </span>
        </label>

        <label className="block">
          <span className="label">Neutral prior</span>
          <input
            type="number"
            step="1"
            min="0"
            max="500"
            value={draft.neutral_prior}
            onChange={(event) => updateField('neutral_prior', event.target.value)}
            className="field mt-1"
            disabled={loading || saving}
          />
          <span className="mt-1 block font-mono text-[8px] uppercase tracking-[0.12em] text-muted/70">
            Higher pulls songs with little evidence toward 1000
          </span>
        </label>

        <label className="block">
          <span className="label">Minimum picks per voter</span>
          <input
            type="number"
            step="1"
            min="0"
            max="10000"
            value={draft.min_picks}
            onChange={(event) => updateField('min_picks', event.target.value)}
            className="field mt-1"
            disabled={loading || saving}
          />
          <span className="mt-1 block font-mono text-[8px] uppercase tracking-[0.12em] text-muted/70">
            0 includes every voter, even one-pick accounts
          </span>
        </label>

        <div className="flex flex-wrap items-center gap-3 sm:col-span-2">
          <button
            type="submit"
            className="btn"
            disabled={loading || saving}
          >
            {saving ? 'Saving…' : 'Save settings'}
          </button>
          {message && <span className="text-sm text-ok">{message}</span>}
        </div>
      </form>
    </section>
  )
}
