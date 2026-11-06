import type { RefObject } from 'react'

import { CassetteHub } from './CassetteHub'
import { DuelSide } from './DuelSide'
import { useCardSwipe } from './useCardSwipe'
import type { Matchup } from '../../lib/types'

type DuelCassetteProps = {
  matchup: Matchup
  pickedId: string | null
  playingSide: 'a' | 'b' | null
  disabled: boolean
  audioA: RefObject<HTMLAudioElement | null>
  audioB: RefObject<HTMLAudioElement | null>
  onPlayA: (playing: boolean) => void
  onPlayB: (playing: boolean) => void
  onTogglePreview: (side: 'a' | 'b') => void
  onEnded: (side: 'a' | 'b') => void
  onPick: (side: 'a' | 'b') => void
}

export function DuelCassette({
  matchup,
  pickedId,
  playingSide,
  disabled,
  audioA,
  audioB,
  onPlayA,
  onPlayB,
  onTogglePreview,
  onEnded,
  onPick,
}: DuelCassetteProps) {
  const { cardRef, dragging, direction, cardProps } = useCardSwipe({
    disabled,
    onPick,
  })

  const pickedSide = pickedId
    ? pickedId === matchup.track_a.id
      ? 'a'
      : pickedId === matchup.track_b.id
        ? 'b'
        : null
    : null

  return (
    <div
      ref={cardRef}
      {...cardProps}
      role="group"
      aria-label={`Duel between ${matchup.track_a.title} and ${matchup.track_b.title}`}
      data-swipe={direction ?? undefined}
      className={`pv-duel-cassette ${dragging ? 'is-dragging' : ''} ${
        pickedSide ? `is-picking-${pickedSide}` : ''
      }`}
    >
      <span className="pv-duel-ghost pv-duel-ghost-a" aria-hidden="true">
        A
      </span>
      <span className="pv-duel-ghost pv-duel-ghost-b" aria-hidden="true">
        B
      </span>

      <div className="pv-duel-sides">
        <DuelSide
          key={`a-${matchup.track_a.id}-${matchup.track_b.id}`}
          side="a"
          track={matchup.track_a}
          picked={pickedSide === 'a'}
          playing={playingSide === 'a'}
          dimmed={pickedSide !== null && pickedSide !== 'a'}
          disabled={disabled}
          audioRef={audioA}
          onPlayStateChange={onPlayA}
          onTogglePreview={() => onTogglePreview('a')}
          onEnded={() => onEnded('a')}
          onPick={() => onPick('a')}
        />

        <CassetteHub playing={playingSide !== null} />

        <DuelSide
          key={`b-${matchup.track_a.id}-${matchup.track_b.id}`}
          side="b"
          track={matchup.track_b}
          picked={pickedSide === 'b'}
          playing={playingSide === 'b'}
          dimmed={pickedSide !== null && pickedSide !== 'b'}
          disabled={disabled}
          audioRef={audioB}
          onPlayStateChange={onPlayB}
          onTogglePreview={() => onTogglePreview('b')}
          onEnded={() => onEnded('b')}
          onPick={() => onPick('b')}
        />
      </div>
    </div>
  )
}
