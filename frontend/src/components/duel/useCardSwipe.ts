import { useCallback, useEffect, useRef, useState } from 'react'
import type { MouseEvent as ReactMouseEvent, PointerEvent as ReactPointerEvent } from 'react'

export type DuelSideId = 'a' | 'b'

type GestureState = {
  pointerId: number
  startX: number
  startY: number
  lastX: number
  lastTime: number
  axis: 'x' | 'y' | null
  dragging: boolean
  velocityX: number
}

const AXIS_LOCK = 14
const PICK_THRESHOLD = 56
const VELOCITY_THRESHOLD = 0.55
const MAX_DRAG = 96
const CLICK_SUPPRESS_MS = 420

type UseCardSwipeOptions = {
  disabled?: boolean
  onPick: (side: DuelSideId) => void
}

/**
 * Horizontal drag-to-pick for the duel cassette.
 *
 * Vertical movement is left to the browser so the page can still scroll.
 * Taps on buttons keep working; only a recognised horizontal gesture suppresses
 * the click that would otherwise follow pointerup.
 */
export function useCardSwipe({ disabled = false, onPick }: UseCardSwipeOptions) {
  const cardRef = useRef<HTMLDivElement | null>(null)
  const gesture = useRef<GestureState | null>(null)
  const suppressClickUntil = useRef(0)
  const [dragging, setDragging] = useState(false)
  const [direction, setDirection] = useState<DuelSideId | null>(null)

  const resetVisual = useCallback(() => {
    const card = cardRef.current
    card?.style.setProperty('--duel-drag', '0px')
    card?.style.setProperty('--duel-tilt', '0deg')
    setDragging(false)
    setDirection(null)
  }, [])

  useEffect(() => {
    if (!disabled) return
    gesture.current = null
    resetVisual()
  }, [disabled, resetVisual])

  const handlePointerDown = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (disabled) return
      if (!event.isPrimary) {
        // A second finger means pinch/zoom, not a pick.
        gesture.current = null
        resetVisual()
        return
      }
      if (event.pointerType === 'mouse' && event.button !== 0) return
      if ((event.target as HTMLElement).closest('[data-no-swipe]')) return

      gesture.current = {
        pointerId: event.pointerId,
        startX: event.clientX,
        startY: event.clientY,
        lastX: event.clientX,
        lastTime: event.timeStamp,
        axis: null,
        dragging: false,
        velocityX: 0,
      }
    },
    [disabled, resetVisual],
  )

  const handlePointerMove = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      const state = gesture.current
      if (!state || state.pointerId !== event.pointerId || disabled) return

      const dx = event.clientX - state.startX
      const dy = event.clientY - state.startY

      if (!state.axis) {
        if (Math.abs(dx) < AXIS_LOCK && Math.abs(dy) < AXIS_LOCK) return

        state.axis = Math.abs(dx) > Math.abs(dy) * 1.1 ? 'x' : 'y'
        if (state.axis === 'y') {
          resetVisual()
          return
        }

        state.dragging = true
        setDragging(true)

        try {
          event.currentTarget.setPointerCapture(event.pointerId)
        } catch {
          // Pointer capture is a progressive enhancement; the gesture still works.
        }
      }

      if (state.axis !== 'x') return

      const elapsed = Math.max(1, event.timeStamp - state.lastTime)
      state.velocityX = (event.clientX - state.lastX) / elapsed
      state.lastX = event.clientX
      state.lastTime = event.timeStamp

      const clamped = Math.max(-MAX_DRAG, Math.min(MAX_DRAG, dx))
      const card = cardRef.current
      card?.style.setProperty('--duel-drag', `${clamped * 0.55}px`)
      card?.style.setProperty('--duel-tilt', `${clamped * 0.018}deg`)

      setDirection(
        dx <= -PICK_THRESHOLD ? 'a' : dx >= PICK_THRESHOLD ? 'b' : null,
      )
    },
    [disabled, resetVisual],
  )

  const finishGesture = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>, canceled: boolean) => {
      const state = gesture.current
      if (!state || state.pointerId !== event.pointerId) return
      gesture.current = null

      const dx = event.clientX - state.startX

      if (!canceled && state.axis === 'x' && state.dragging) {
        const shouldPick =
          Math.abs(dx) >= PICK_THRESHOLD ||
          Math.abs(state.velocityX) >= VELOCITY_THRESHOLD

        // A horizontal drag should never double as a button tap, even when it
        // ends up below the pick threshold.
        suppressClickUntil.current = performance.now() + CLICK_SUPPRESS_MS

        if (shouldPick) {
          const side: DuelSideId = dx < 0 ? 'a' : 'b'
          resetVisual()
          onPick(side)
          return
        }
      }

      resetVisual()
    },
    [onPick, resetVisual],
  )

  const handleClickCapture = useCallback(
    (event: ReactMouseEvent<HTMLDivElement>) => {
      if (performance.now() > suppressClickUntil.current) return
      event.preventDefault()
      event.stopPropagation()
    },
    [],
  )

  return {
    cardRef,
    dragging,
    direction,
    cardProps: {
      onPointerDown: handlePointerDown,
      onPointerMove: handlePointerMove,
      onPointerUp: (event: ReactPointerEvent<HTMLDivElement>) =>
        finishGesture(event, false),
      onPointerCancel: (event: ReactPointerEvent<HTMLDivElement>) =>
        finishGesture(event, true),
      onClickCapture: handleClickCapture,
    },
  }
}
