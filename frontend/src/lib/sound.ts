const STORAGE_KEY = 'pv-sound'
let cached: boolean | null = null
let audioContext: AudioContext | null = null

export function isSoundEnabled(): boolean {
  if (cached !== null) return cached

  try {
    cached = window.localStorage.getItem(STORAGE_KEY) === 'on'
  } catch {
    cached = false
  }

  return cached
}

export function setSoundEnabled(enabled: boolean) {
  cached = enabled

  try {
    window.localStorage.setItem(STORAGE_KEY, enabled ? 'on' : 'off')
  } catch {
    // Storage can be disabled; the in-memory preference still works.
  }
}

/** A tiny, warm "stamp" for picks. No audio files, no dependencies. */
export function playStamp() {
  if (!isSoundEnabled()) return

  const AudioConstructor =
    window.AudioContext ??
    (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext

  if (!AudioConstructor) return

  audioContext ??= new AudioConstructor()
  const context = audioContext
  const now = context.currentTime

  void context.resume()

  const thump = context.createOscillator()
  const thumpGain = context.createGain()
  thump.type = 'sine'
  thump.frequency.setValueAtTime(165, now)
  thump.frequency.exponentialRampToValueAtTime(72, now + 0.09)
  thumpGain.gain.setValueAtTime(0.0001, now)
  thumpGain.gain.exponentialRampToValueAtTime(0.14, now + 0.008)
  thumpGain.gain.exponentialRampToValueAtTime(0.0001, now + 0.15)
  thump.connect(thumpGain).connect(context.destination)
  thump.start(now)
  thump.stop(now + 0.16)

  const click = context.createOscillator()
  const clickGain = context.createGain()
  click.type = 'square'
  click.frequency.setValueAtTime(920, now)
  clickGain.gain.setValueAtTime(0.028, now)
  clickGain.gain.exponentialRampToValueAtTime(0.0001, now + 0.03)
  click.connect(clickGain).connect(context.destination)
  click.start(now)
  click.stop(now + 0.04)
}
