const STORAGE_KEY = 'pv-sound'
const SOUND_LEVEL = 6
let cached: boolean | null = null
let audioContext: AudioContext | null = null
let masterBus: GainNode | null = null

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

type ToneOptions = {
  frequency: number
  endFrequency?: number
  type?: OscillatorType
  duration?: number
  delay?: number
  gain?: number
  pan?: number
}

function soundContext(): AudioContext | null {
  if (!isSoundEnabled()) return null

  const AudioConstructor =
    window.AudioContext ??
    (window as unknown as { webkitAudioContext?: typeof AudioContext })
      .webkitAudioContext

  if (!AudioConstructor) return null

  audioContext ??= new AudioConstructor()
  void audioContext.resume()
  return audioContext
}

/**
 * A shared master bus with a gentle limiter. The synth sounds are short, so a
 * little drive makes them cut through the music without hard clipping.
 */
function outputBus(context: AudioContext): AudioNode {
  if (masterBus) return masterBus

  const compressor = context.createDynamicsCompressor()
  compressor.threshold.value = -14
  compressor.knee.value = 18
  compressor.ratio.value = 6
  compressor.attack.value = 0.003
  compressor.release.value = 0.25

  const bus = context.createGain()
  bus.gain.value = 1.6
  bus.connect(compressor)
  compressor.connect(context.destination)
  masterBus = bus
  return masterBus
}

function noiseClick(
  context: AudioContext,
  options: {
    gain: number
    duration?: number
    highpass?: number
    pan?: number
  },
) {
  const { gain, duration = 0.045, highpass = 1000, pan = 0 } = options
  const length = Math.max(1, Math.floor(context.sampleRate * duration))
  const buffer = context.createBuffer(1, length, context.sampleRate)
  const data = buffer.getChannelData(0)

  for (let index = 0; index < length; index += 1) {
    const envelope = 1 - index / length
    data[index] = (Math.random() * 2 - 1) * envelope * envelope
  }

  const source = context.createBufferSource()
  source.buffer = buffer

  const filter = context.createBiquadFilter()
  filter.type = 'highpass'
  filter.frequency.value = highpass

  const gainNode = context.createGain()
  gainNode.gain.value = Math.min(0.5, Math.max(0.0001, gain * SOUND_LEVEL))

  let output: AudioNode = gainNode
  source.connect(filter)
  filter.connect(gainNode)

  if (typeof context.createStereoPanner === 'function') {
    const panner = context.createStereoPanner()
    panner.pan.value = Math.max(-1, Math.min(1, pan))
    gainNode.connect(panner)
    output = panner
  }

  output.connect(outputBus(context))
  source.start()
  source.stop(context.currentTime + duration + 0.02)
}

type NoiseSweepOptions = {
  from: number
  to: number
  duration?: number
  delay?: number
  gain: number
  pan?: number
  q?: number
  filterType?: BiquadFilterType
}

/** Filtered noise with a moving centre — tape hiss, rewind whirr, motor spin. */
function noiseSweep(context: AudioContext, options: NoiseSweepOptions) {
  const {
    from,
    to,
    duration = 0.2,
    delay = 0,
    gain,
    pan = 0,
    q = 0.9,
    filterType = 'bandpass',
  } = options

  const start = context.currentTime + delay
  const length = Math.max(1, Math.floor(context.sampleRate * duration))
  const buffer = context.createBuffer(1, length, context.sampleRate)
  const data = buffer.getChannelData(0)

  for (let index = 0; index < length; index += 1) {
    const progress = index / length
    const attack = Math.min(1, progress / 0.08)
    const release = 1 - progress
    data[index] =
      (Math.random() * 2 - 1) * attack * release * release
  }

  const source = context.createBufferSource()
  source.buffer = buffer

  const filter = context.createBiquadFilter()
  filter.type = filterType
  filter.Q.value = q
  filter.frequency.setValueAtTime(Math.max(40, from), start)
  filter.frequency.exponentialRampToValueAtTime(
    Math.max(40, to),
    start + duration,
  )

  const gainNode = context.createGain()
  gainNode.gain.setValueAtTime(0.0001, start)
  gainNode.gain.exponentialRampToValueAtTime(
    Math.min(0.4, Math.max(0.0001, gain * SOUND_LEVEL)),
    start + 0.015,
  )
  gainNode.gain.exponentialRampToValueAtTime(0.0001, start + duration)

  source.connect(filter)
  filter.connect(gainNode)

  let output: AudioNode = gainNode
  if (typeof context.createStereoPanner === 'function') {
    const panner = context.createStereoPanner()
    panner.pan.value = Math.max(-1, Math.min(1, pan))
    gainNode.connect(panner)
    output = panner
  }

  output.connect(outputBus(context))
  source.start(start)
  source.stop(start + duration + 0.02)
}

type PluckOptions = {
  frequency: number
  duration?: number
  delay?: number
  gain: number
  pan?: number
  type?: OscillatorType
  filter?: number
}

/** A warm, slightly detuned pluck — the musical part of a pick or chime. */
function pluck(context: AudioContext, options: PluckOptions) {
  const {
    frequency,
    duration = 0.28,
    delay = 0,
    gain,
    pan = 0,
    type = 'triangle',
    filter = 2800,
  } = options

  const start = context.currentTime + delay
  const root = context.createOscillator()
  const shimmer = context.createOscillator()
  const shimmerGain = context.createGain()
  const filterNode = context.createBiquadFilter()
  const envelope = context.createGain()

  root.type = type
  root.frequency.setValueAtTime(frequency, start)
  shimmer.type = 'sine'
  shimmer.frequency.setValueAtTime(frequency * 2, start)
  shimmerGain.gain.value = 0.16

  filterNode.type = 'lowpass'
  filterNode.Q.value = 0.7
  filterNode.frequency.setValueAtTime(filter, start)
  filterNode.frequency.exponentialRampToValueAtTime(
    Math.max(260, filter * 0.35),
    start + duration,
  )

  envelope.gain.setValueAtTime(0.0001, start)
  envelope.gain.exponentialRampToValueAtTime(
    Math.min(0.5, Math.max(0.0001, gain * SOUND_LEVEL)),
    start + 0.008,
  )
  envelope.gain.exponentialRampToValueAtTime(0.0001, start + duration)

  root.connect(filterNode)
  shimmer.connect(shimmerGain)
  shimmerGain.connect(filterNode)
  filterNode.connect(envelope)

  let output: AudioNode = envelope
  if (typeof context.createStereoPanner === 'function') {
    const panner = context.createStereoPanner()
    panner.pan.value = Math.max(-1, Math.min(1, pan))
    envelope.connect(panner)
    output = panner
  }

  output.connect(outputBus(context))
  root.start(start)
  shimmer.start(start)
  root.stop(start + duration + 0.02)
  shimmer.stop(start + duration + 0.02)
}

function tone(context: AudioContext, options: ToneOptions) {
  const {
    frequency,
    endFrequency = frequency,
    type = 'sine',
    duration = 0.12,
    delay = 0,
    gain: peak = 0.05,
    pan = 0,
  } = options

  const start = context.currentTime + delay
  const oscillator = context.createOscillator()
  const gain = context.createGain()
  oscillator.type = type
  oscillator.frequency.setValueAtTime(frequency, start)
  oscillator.frequency.exponentialRampToValueAtTime(
    Math.max(20, endFrequency),
    start + duration,
  )

  gain.gain.setValueAtTime(0.0001, start)
  gain.gain.exponentialRampToValueAtTime(
    Math.min(0.5, Math.max(0.0001, peak * SOUND_LEVEL)),
    start + 0.008,
  )
  gain.gain.exponentialRampToValueAtTime(0.0001, start + duration)

  let output: AudioNode = gain
  oscillator.connect(gain)

  if (typeof context.createStereoPanner === 'function') {
    const panner = context.createStereoPanner()
    panner.pan.value = Math.max(-1, Math.min(1, pan))
    gain.connect(panner)
    output = panner
  }

  output.connect(outputBus(context))
  oscillator.start(start)
  oscillator.stop(start + duration + 0.02)
}

/** The mechanical tape-key press, played immediately on a pick. */
export function playPickPress(side?: 'a' | 'b') {
  const context = soundContext()
  if (!context) return
  const pan = side === 'a' ? -0.12 : side === 'b' ? 0.12 : 0

  noiseClick(context, { gain: 0.045, duration: 0.035, highpass: 1700, pan })
  noiseSweep(context, {
    from: 1400,
    to: 650,
    duration: 0.07,
    gain: 0.03,
    pan,
    q: 0.8,
  })
  tone(context, {
    frequency: 170,
    endFrequency: 100,
    duration: 0.09,
    gain: 0.035,
    pan,
  })
}

/** The warm musical confirmation, played once the pick has saved. */
export function playPickConfirm(side?: 'a' | 'b') {
  const context = soundContext()
  if (!context) return
  const pan = side === 'a' ? -0.12 : side === 'b' ? 0.12 : 0
  const root = (side === 'a' ? 523.25 : 783.99) * (1 + (Math.random() - 0.5) * 0.014)
  const fifth = root * 1.5

  // A is C, B is G; each gets a soft fifth.
  pluck(context, {
    frequency: root,
    duration: 0.3,
    delay: 0.01,
    gain: 0.055,
    pan,
    filter: 2700,
  })
  pluck(context, {
    frequency: fifth,
    duration: 0.22,
    delay: 0.03,
    gain: 0.024,
    pan: pan * 0.7,
    filter: 3200,
  })

  // A little tape tail so it feels like the transport is still settling.
  noiseSweep(context, {
    from: 2200,
    to: 500,
    duration: 0.12,
    delay: 0.04,
    gain: 0.01,
    pan,
    q: 0.7,
  })
}

/** Combined preview for contexts that want the whole pick gesture. */
export function playPick(side?: 'a' | 'b') {
  playPickPress(side)
  playPickConfirm(side)
}

/** A tape rewind for undoing a pick: the mechanical opposite of a pick. */
export function playUndo() {
  const context = soundContext()
  if (!context) return

  noiseSweep(context, {
    from: 2600,
    to: 600,
    duration: 0.3,
    gain: 0.04,
    q: 0.8,
  })
  tone(context, {
    frequency: 240,
    endFrequency: 90,
    duration: 0.26,
    gain: 0.04,
  })
  tone(context, {
    frequency: 480,
    endFrequency: 300,
    duration: 0.12,
    delay: 0.12,
    gain: 0.018,
  })
  noiseClick(context, { gain: 0.015, duration: 0.025, highpass: 1100 })
}

/** A transport key for auto-preview: spin up when on, wind down when off. */
export function playToggle(on: boolean) {
  const context = soundContext()
  if (!context) return

  noiseClick(context, {
    gain: 0.035,
    duration: 0.03,
    highpass: 1800,
  })

  if (on) {
    noiseSweep(context, {
      from: 500,
      to: 2200,
      duration: 0.18,
      gain: 0.018,
      q: 1.1,
    })
    pluck(context, {
      frequency: 523.25,
      duration: 0.2,
      delay: 0.035,
      gain: 0.035,
      filter: 2800,
    })
    pluck(context, {
      frequency: 783.99,
      duration: 0.18,
      delay: 0.095,
      gain: 0.022,
      filter: 3200,
    })
  } else {
    noiseSweep(context, {
      from: 2200,
      to: 500,
      duration: 0.16,
      gain: 0.015,
      q: 1.1,
    })
    pluck(context, {
      frequency: 783.99,
      duration: 0.16,
      delay: 0.03,
      gain: 0.022,
      filter: 3200,
    })
    pluck(context, {
      frequency: 523.25,
      duration: 0.18,
      delay: 0.085,
      gain: 0.017,
      filter: 2600,
    })
  }
}

/** A gentle rising chime for completing today's streak goal. */
export function playStreakGoal() {
  const context = soundContext()
  if (!context) return

  // Tape latch, then a little hiss swell underneath the chime.
  noiseClick(context, { gain: 0.025, duration: 0.03, highpass: 900 })
  tone(context, {
    frequency: 160,
    endFrequency: 100,
    duration: 0.12,
    gain: 0.035,
  })
  noiseSweep(context, {
    from: 900,
    to: 2600,
    duration: 0.45,
    gain: 0.012,
    q: 0.7,
  })

  const notes = [523.25, 659.25, 783.99, 1046.5]
  notes.forEach((frequency, index) => {
    pluck(context, {
      frequency: frequency * (1 + (Math.random() - 0.5) * 0.006),
      duration: index === notes.length - 1 ? 0.45 : 0.35,
      delay: 0.02 + index * 0.07,
      gain: 0.05 - index * 0.004,
      pan: index % 2 === 0 ? -0.08 : 0.08,
      filter: 3200,
    })
  })

  tone(context, {
    frequency: 261.63,
    type: 'sine',
    duration: 0.4,
    gain: 0.02,
  })
}

/** A larger, still restrained tape-deck fanfare for streak milestones. */
export function playStreakMilestone() {
  const context = soundContext()
  if (!context) return

  // A short rewind/riser makes the milestone feel like it is arriving.
  noiseSweep(context, {
    from: 500,
    to: 3200,
    duration: 0.28,
    gain: 0.022,
    q: 0.9,
  })
  tone(context, {
    frequency: 110,
    endFrequency: 170,
    duration: 0.3,
    gain: 0.025,
  })

  const notes = [261.63, 329.63, 392, 523.25, 659.25, 783.99]
  notes.forEach((frequency, index) => {
    pluck(context, {
      frequency: frequency * (1 + (Math.random() - 0.5) * 0.005),
      duration: 0.48,
      delay: 0.18 + index * 0.07,
      gain: 0.052 - index * 0.004,
      pan: index % 2 === 0 ? -0.1 : 0.1,
      filter: 3400,
    })
  })

  // A little extra shimmer and low warmth for the big moments.
  tone(context, {
    frequency: 130.81,
    type: 'sine',
    duration: 0.6,
    gain: 0.025,
  })
  tone(context, {
    frequency: 1567.98,
    type: 'sine',
    duration: 0.2,
    delay: 0.55,
    gain: 0.012,
  })
  noiseSweep(context, {
    from: 2200,
    to: 700,
    duration: 0.35,
    delay: 0.25,
    gain: 0.012,
    q: 0.7,
  })
}

/** A tiny, warm "stamp" for the sound toggle preview. */
export function playStamp() {
  const context = soundContext()
  if (!context) return

  noiseClick(context, { gain: 0.04, duration: 0.035, highpass: 1400 })
  tone(context, {
    frequency: 180,
    endFrequency: 90,
    duration: 0.12,
    gain: 0.06,
  })
  pluck(context, {
    frequency: 523.25,
    duration: 0.2,
    delay: 0.01,
    gain: 0.04,
    filter: 2600,
  })
}
