type CassetteHubProps = {
  playing: boolean
}

export function CassetteHub({ playing }: CassetteHubProps) {
  return (
    <div className="pv-duel-hub" aria-hidden="true">
      <span className="pv-duel-hub-line" />
      <span className={`pv-duel-reel ${playing ? 'is-spinning' : ''}`} />
      <span className="pv-duel-hub-label">VS</span>
      <span className={`pv-duel-reel ${playing ? 'is-spinning' : ''}`} />
      <span className="pv-duel-hub-line" />
    </div>
  )
}
