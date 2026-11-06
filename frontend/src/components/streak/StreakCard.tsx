import { Link } from 'react-router-dom'

import { StreakDots } from './StreakDots'
import { StreakFlame } from './StreakFlame'
import type { StreakSummary } from '../../lib/types'

type StreakCardProps = {
  streak: StreakSummary | null
  loading?: boolean
}

export function StreakCard({ streak, loading = false }: StreakCardProps) {
  if (loading) {
    return (
      <section
        className="pv-streak-card pv-streak-card-loading"
        aria-hidden="true"
      >
        <span className="pv-streak-card-skel pv-streak-card-skel-flame" />
        <span className="pv-streak-card-skel pv-streak-card-skel-number" />
        <span className="pv-streak-card-skel pv-streak-card-skel-line" />
      </section>
    )
  }

  if (!streak) return null

  const remaining = Math.max(0, streak.goal - streak.today_picks)
  const copy = streak.today_met
    ? "Today’s in the books. Come back tomorrow to keep the flame going."
    : streak.current > 0
      ? `${remaining} more ${remaining === 1 ? 'pick' : 'picks'} today to keep the streak alive.`
      : streak.today_picks > 0
        ? `${remaining} more ${remaining === 1 ? 'pick' : 'picks'} to start your streak.`
        : `Make ${streak.goal} picks today to start your streak.`
  const cta = streak.today_met
    ? 'Next duel'
    : streak.current > 0
      ? 'Keep it alive'
      : streak.today_picks > 0
        ? 'Finish it'
        : 'Start today'
  const streakLabel =
    streak.current > 0 ? `${streak.current} day streak` : 'No active streak'

  return (
    <section
      className={`pv-streak-card ${streak.at_risk ? 'is-at-risk' : ''} ${
        streak.today_met ? 'is-done' : ''
      }`}
      aria-label={`${streakLabel}. ${copy}`}
    >
      {streak.today_met && (
        <span className="pv-streak-stamp" aria-hidden="true">
          Goal met
        </span>
      )}

      <div className="pv-streak-card-main">
        <span className="pv-streak-flame">
          <StreakFlame className="size-7" />
        </span>
        <div className="min-w-0">
          <p className="pv-label">Daily streak</p>
          <p className="pv-streak-card-number">
            {streak.current > 0 ? (
              <>
                <strong>{streak.current}</strong>
                <span>day streak</span>
              </>
            ) : (
              <span className="pv-streak-card-start">Start your streak</span>
            )}
          </p>
          <p className="pv-streak-card-copy">{copy}</p>
        </div>
      </div>

      <div className="pv-streak-card-today">
        <p className="pv-label">Today</p>
        <div
          className="pv-streak-progress"
          role="progressbar"
          aria-label="Today's streak progress"
          aria-valuemin={0}
          aria-valuemax={streak.goal}
          aria-valuenow={Math.min(streak.today_picks, streak.goal)}
        >
          <span
            style={{
              width: `${Math.min(
                100,
                (streak.today_picks / streak.goal) * 100,
              )}%`,
            }}
          />
        </div>
        <p className="pv-streak-progress-label">
          {streak.today_met
            ? `${streak.today_picks} ${streak.today_picks === 1 ? 'pick' : 'picks'} today`
            : `${streak.today_picks} / ${streak.goal} picks`}
        </p>
      </div>

      <div className="pv-streak-card-week">
        <p className="pv-label">Last 7 days</p>
        <StreakDots days={streak.days} showLabels />
      </div>

      <div className="pv-streak-card-side">
        <span className="pv-streak-best">Best {streak.longest}</span>
        <Link to="/duel" className="pv-streak-cta">
          {cta} →
        </Link>
      </div>
    </section>
  )
}
