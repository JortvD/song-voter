export type PlaylistAccent = {
  accent: string
  strong: string
  ink: string
  soft: string
  tint: string
}

export const FALLBACK_ACCENT: PlaylistAccent = {
  accent: '#ff5a1f',
  strong: '#a33a14',
  ink: '#141a2a',
  soft: 'rgba(255, 90, 31, 0.16)',
  tint: 'rgba(255, 90, 31, 0.08)',
}

/** Write the playlist accent into the CSS variables used across the site. */
export function applyPlaylistAccent(value: PlaylistAccent | null) {
  if (typeof document === 'undefined') return

  const accent = value ?? FALLBACK_ACCENT
  const root = document.documentElement

  root.style.setProperty('--playlist-accent', accent.accent)
  root.style.setProperty('--playlist-accent-strong', accent.strong)
  root.style.setProperty('--playlist-accent-ink', accent.ink)
  root.style.setProperty('--playlist-accent-soft', accent.soft)
  root.style.setProperty('--playlist-accent-tint', accent.tint)

  applyFaviconAccent(accent)
}

/** Keep the tab icon's tape card in sync with the current album's accent. */
function applyFaviconAccent(accent: PlaylistAccent) {
  const link = document.querySelector<HTMLLinkElement>('link[rel="icon"]')
  if (!link) return

  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 44 44">
  <rect x="4" y="8" width="23" height="28" rx="5" transform="rotate(-7 15.5 22)" fill="#f7eedf" stroke="#141a2a" stroke-width="2" />
  <rect x="17" y="8" width="23" height="28" rx="5" transform="rotate(7 28.5 22)" fill="${accent.accent}" stroke="#141a2a" stroke-width="2" />
  <text x="12.5" y="27" text-anchor="middle" font-family="'Bricolage Grotesque', 'DM Sans', sans-serif" font-size="14" font-weight="700" fill="#141a2a">A</text>
  <text x="31.5" y="27" text-anchor="middle" font-family="'Bricolage Grotesque', 'DM Sans', sans-serif" font-size="14" font-weight="700" fill="${accent.ink}">B</text>
</svg>`

  link.type = 'image/svg+xml'
  link.href = `data:image/svg+xml,${encodeURIComponent(svg)}`
}

/**
 * Sample a readable accent color from the playlist cover.
 *
 * The image is drawn to a tiny canvas and the most vivid hue family wins. If
 * the browser refuses the cross-origin image or the cover is monochrome, the
 * caller falls back to safety orange.
 */
export function extractPlaylistAccent(imageUrl: string): Promise<PlaylistAccent | null> {
  return new Promise((resolve) => {
    if (!imageUrl) {
      resolve(null)
      return
    }

    const image = new Image()
    image.crossOrigin = 'anonymous'
    image.decoding = 'async'

    image.onload = () => {
      try {
        const size = 40
        const canvas = document.createElement('canvas')
        canvas.width = size
        canvas.height = size

        const context = canvas.getContext('2d', { willReadFrequently: true })
        if (!context) {
          resolve(null)
          return
        }

        context.drawImage(image, 0, 0, size, size)
        const pixels = context.getImageData(0, 0, size, size).data

        const buckets = Array.from({ length: 12 }, () => ({
          weight: 0,
          red: 0,
          green: 0,
          blue: 0,
        }))

        for (let index = 0; index < pixels.length; index += 4) {
          if (pixels[index + 3] < 200) continue

          const red = pixels[index]
          const green = pixels[index + 1]
          const blue = pixels[index + 2]
          const { h, s, l } = rgbToHsl(red, green, blue)

          if (s < 0.2 || l < 0.1 || l > 0.9) continue

          const bucket = buckets[Math.min(11, Math.floor(h / 30))]
          const weight = s * (1 - Math.abs(l - 0.5))

          bucket.weight += weight
          bucket.red += red * weight
          bucket.green += green * weight
          bucket.blue += blue * weight
        }

        const best = buckets.reduce((winner, bucket) =>
          bucket.weight > winner.weight ? bucket : winner,
        )

        if (best.weight <= 0) {
          resolve(null)
          return
        }

        resolve(
          normalizeAccent(
            Math.round(best.red / best.weight),
            Math.round(best.green / best.weight),
            Math.round(best.blue / best.weight),
          ),
        )
      } catch {
        // Tainted canvas or a decode problem: use the brand fallback.
        resolve(null)
      }
    }

    image.onerror = () => resolve(null)
    image.src = imageUrl
  })
}

function normalizeAccent(red: number, green: number, blue: number): PlaylistAccent {
  const hsl = rgbToHsl(red, green, blue)
  const saturation = clamp(hsl.s, 0.55, 0.9)
  const lightness = clamp(hsl.l, 0.42, 0.58)
  const [nextRed, nextGreen, nextBlue] = hslToRgb(hsl.h, saturation, lightness)
  const accent = rgbToHex(nextRed, nextGreen, nextBlue)
  const strong = darkenForContrast(nextRed, nextGreen, nextBlue, hsl.h, saturation, lightness)
  const inkContrast = contrastRatio(nextRed, nextGreen, nextBlue, 20, 26, 42)
  const creamContrast = contrastRatio(nextRed, nextGreen, nextBlue, 255, 250, 240)
  const ink = inkContrast >= creamContrast ? '#141a2a' : '#fffaf0'

  return {
    accent,
    strong,
    ink,
    soft: `rgba(${nextRed}, ${nextGreen}, ${nextBlue}, 0.16)`,
    tint: `rgba(${nextRed}, ${nextGreen}, ${nextBlue}, 0.08)`,
  }
}

/** Darken the accent until it is readable as text on the cream paper. */
function darkenForContrast(
  red: number,
  green: number,
  blue: number,
  hue: number,
  saturation: number,
  lightness: number,
): string {
  let currentLightness = lightness
  let [nextRed, nextGreen, nextBlue] = [red, green, blue]

  while (
    contrastRatio(nextRed, nextGreen, nextBlue, 247, 238, 223) < 4.5 &&
    currentLightness > 0.12
  ) {
    currentLightness -= 0.02
    ;[nextRed, nextGreen, nextBlue] = hslToRgb(hue, saturation, currentLightness)
  }

  return rgbToHex(nextRed, nextGreen, nextBlue)
}

function contrastRatio(
  red: number,
  green: number,
  blue: number,
  otherRed: number,
  otherGreen: number,
  otherBlue: number,
): number {
  const first = relativeLuminance(red, green, blue)
  const second = relativeLuminance(otherRed, otherGreen, otherBlue)
  const lighter = Math.max(first, second)
  const darker = Math.min(first, second)
  return (lighter + 0.05) / (darker + 0.05)
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value))
}

function rgbToHsl(red: number, green: number, blue: number) {
  const r = red / 255
  const g = green / 255
  const b = blue / 255
  const maximum = Math.max(r, g, b)
  const minimum = Math.min(r, g, b)
  const lightness = (maximum + minimum) / 2

  if (maximum === minimum) {
    return { h: 0, s: 0, l: lightness }
  }

  const delta = maximum - minimum
  const saturation =
    lightness > 0.5 ? delta / (2 - maximum - minimum) : delta / (maximum + minimum)

  let hue: number
  switch (maximum) {
    case r:
      hue = (g - b) / delta + (g < b ? 6 : 0)
      break
    case g:
      hue = (b - r) / delta + 2
      break
    default:
      hue = (r - g) / delta + 4
      break
  }

  return { h: hue * 60, s: saturation, l: lightness }
}

function hslToRgb(hue: number, saturation: number, lightness: number): [number, number, number] {
  const h = ((hue % 360) + 360) % 360
  const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation
  const x = chroma * (1 - Math.abs(((h / 60) % 2) - 1))
  const m = lightness - chroma / 2

  let r = 0
  let g = 0
  let b = 0

  if (h < 60) {
    ;[r, g, b] = [chroma, x, 0]
  } else if (h < 120) {
    ;[r, g, b] = [x, chroma, 0]
  } else if (h < 180) {
    ;[r, g, b] = [0, chroma, x]
  } else if (h < 240) {
    ;[r, g, b] = [0, x, chroma]
  } else if (h < 300) {
    ;[r, g, b] = [x, 0, chroma]
  } else {
    ;[r, g, b] = [chroma, 0, x]
  }

  return [
    Math.round((r + m) * 255),
    Math.round((g + m) * 255),
    Math.round((b + m) * 255),
  ]
}

function rgbToHex(red: number, green: number, blue: number): string {
  return `#${[red, green, blue]
    .map((channel) => channel.toString(16).padStart(2, '0'))
    .join('')}`
}

function relativeLuminance(red: number, green: number, blue: number): number {
  return (
    0.2126 * linearChannel(red) +
    0.7152 * linearChannel(green) +
    0.0722 * linearChannel(blue)
  )
}

function linearChannel(value: number): number {
  const channel = value / 255
  return channel <= 0.03928 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4
}
