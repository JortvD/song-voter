import { StreakDots } from '../streak/StreakDots'
import { StreakFlame } from '../streak/StreakFlame'
import type { StreakSummary } from '../../lib/types'

export function StreakBar({ streak }: { streak: StreakSummary }) {
  const remaining = Math.max(0, streak.goal - streak.today_picks)
  const todayLabel = streak.today_met
    ? 'Today done'
    : `${streak.today_picks}/${streak.goal} today`
  const aria = streak.today_met
    ? `${streak.current} day streak. Today complete.`
    : streak.current > 0
      ? `${streak.current} day streak. ${remaining} more ${
          remaining === 1 ? 'pick' : 'picks'
        } today to keep it alive.`
      : `Start a streak. ${streak.today_picks} of ${streak.goal} picks today.`

  return (
    <section
      className={`pv-streak-bar ${streak.at_risk ? 'is-at-risk' : ''} ${
        streak.today_met ? 'is-done' : ''
      }`}
      aria-label={aria}
    >
      <span className="pv-streak-flame">
        <StreakFlame className="size-5" />
      </span>

      <span className="pv-streak-bar-main">
        {streak.current > 0 ? (
          <>
            <strong>{streak.current}</strong>
            <span>day streak</span>
          </>
        ) : (
          <span className="pv-streak-start">Start a streak</span>
        )}
      </span>

      <span className="pv-streak-bar-today">
        {streak.today_met ? '✓ Today done' : todayLabel}
      </span>

      <StreakDots days={streak.days} />
    </section>
  )
}
