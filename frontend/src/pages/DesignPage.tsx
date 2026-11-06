import { useRef } from 'react'
import type { ReactNode } from 'react'

import { BrandMark } from '../components/BrandMark'
import { MixtapeCase } from '../components/MixtapeCase'
import { TapeCard } from '../components/TapeCard'
import type { MatchupTrack } from '../lib/types'

const sampleA: MatchupTrack = {
  id: 'sample-a',
  spotify_id: 'sample-a',
  title: 'Midnight City',
  artists: 'M83',
  album: 'Hurry Up, We’re Dreaming',
  album_image_url: null,
  preview_url: null,
  duration_ms: 244000,
}

const sampleB: MatchupTrack = {
  id: 'sample-b',
  spotify_id: 'sample-b',
  title: 'Dreams',
  artists: 'Fleetwood Mac',
  album: 'Rumours',
  album_image_url: null,
  preview_url: null,
  duration_ms: 257000,
}

const swatches = [
  { name: 'Paper', value: '#f7eedf' },
  { name: 'Ink', value: '#141a2a' },
  { name: 'Cobalt', value: '#2450d8' },
  { name: 'Safety orange', value: '#ff5a1f' },
  { name: 'Butter', value: '#f2c75c' },
]

function Section({
  number,
  title,
  children,
}: {
  number: string
  title: string
  children: ReactNode
}) {
  return (
    <section className="border-t-2 border-ink/15 py-8">
      <div className="flex items-baseline gap-3">
        <span className="font-serif text-3xl leading-none text-[var(--playlist-accent-strong)]">
          {number}
        </span>
        <h2 className="font-serif text-2xl tracking-[-0.03em]">{title}</h2>
      </div>
      <div className="mt-5">{children}</div>
    </section>
  )
}

export function DesignPage() {
  const audioA = useRef<HTMLAudioElement | null>(null)
  const audioB = useRef<HTMLAudioElement | null>(null)

  return (
    <main className="mx-auto w-full max-w-6xl flex-1 px-4 pb-20 pt-8 sm:px-6 lg:px-8">
      <div className="flex flex-wrap items-end justify-between gap-3 border-b-2 border-ink/15 pb-4">
        <div>
          <p className="pv-label">Temporary · Phase 1</p>
          <h1 className="mt-1 font-serif text-4xl leading-none tracking-[-0.04em] sm:text-5xl">
            The style tile.
          </h1>
          <p className="mt-2 max-w-xl text-sm leading-relaxed text-muted">
            The new mixtape-zine language: paper, ink, cobalt, safety orange,
            rounded display type, stickers, tape and hard shadows. This route is
            here for review and will be removed before launch.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <BrandMark className="size-12" />
          <span className="pv-sticker">Phase 1</span>
        </div>
      </div>

      <Section number="01" title="Palette">
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
          {swatches.map((swatch) => (
            <div key={swatch.name} className="pv-card overflow-hidden">
              <div className="h-20 border-b-2 border-ink" style={{ background: swatch.value }} />
              <div className="p-3">
                <p className="pv-label text-[9px]">{swatch.name}</p>
                <p className="mt-1 font-mono text-[9px] text-muted">{swatch.value}</p>
              </div>
            </div>
          ))}
        </div>
        <p className="mt-3 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
          Playlist accent: <span className="text-ink">var(--playlist-accent)</span> · sampled
          from the cover with an orange fallback
        </p>
      </Section>

      <Section number="02" title="Type">
        <div className="grid gap-5 lg:grid-cols-3">
          <div className="pv-card p-5">
            <p className="pv-label">Display · Bricolage Grotesque</p>
            <p className="mt-3 font-serif text-6xl leading-[0.86] tracking-[-0.06em]">
              Two songs.
              <br />
              One pick.
            </p>
            <p className="mt-4 text-sm text-muted">
              Rounded, friendly and a little off-centre. Used for headlines and
              big numbers.
            </p>
          </div>
          <div className="pv-card p-5">
            <p className="pv-label">Body · DM Sans</p>
            <p className="mt-3 text-lg leading-relaxed">
              A playlist becomes a contest. Two tracks enter the deck, and every
              pick nudges the shared standings.
            </p>
            <p className="mt-4 text-sm leading-relaxed text-muted">
              Clean enough for long copy, dense tables and mobile screens.
            </p>
          </div>
          <div className="pv-card p-5">
            <p className="pv-label">Labels · IBM Plex Mono</p>
            <p className="mt-3 font-mono text-[11px] uppercase leading-relaxed tracking-[0.16em] text-muted">
              Side A · 31 songs
              <br />
              Picks 1,204 · Duels 8,912
              <br />
              Now voting · Sealed until The Reveal
            </p>
            <div className="mt-4 flex gap-2">
              <span className="pv-key">Space</span>
              <span className="pv-key">1</span>
              <span className="pv-key">2</span>
              <span className="pv-key">Z</span>
            </div>
          </div>
        </div>
      </Section>

      <Section number="03" title="Buttons & stickers">
        <div className="flex flex-wrap items-center gap-3">
          <button type="button" className="btn">
            Start a duel
          </button>
          <button type="button" className="btn-ghost">
            The Standings
          </button>
          <span className="pv-sticker">Now voting</span>
          <span className="pv-sticker pv-sticker-paper">Side A</span>
          <span className="pv-sticker pv-sticker-cobalt">Official</span>
          <span className="pv-sticker pv-sticker-butter">5 in a row</span>
          <span className="pv-key">1</span>
        </div>
      </Section>

      <Section number="04" title="The mixtape case">
        <div className="grid gap-8 lg:grid-cols-[minmax(0,360px)_1fr] lg:items-center">
          <MixtapeCase
            title="The group-chat tape"
            coverUrl={null}
            trackCount={42}
            description="A shared playlist, a stack of duels, and one group’s taste on the line."
            revealed={false}
          />
          <div className="max-w-lg">
            <p className="text-sm leading-relaxed text-muted">
              The home hero is a cassette J-card. The playlist cover becomes the
              label; the accent color is sampled from it. The “Sealed” sticker
              flips to “Standings live” after The Reveal.
            </p>
            <p className="mt-4 font-mono text-[9px] uppercase leading-relaxed tracking-[0.14em] text-muted">
              Paper grain · hard offset shadow · rotated sticker · corner screws
            </p>
          </div>
        </div>
      </Section>

      <Section number="05" title="The tape deck">
        <div className="pv-deck">
          <div className="pv-deck-top">
            <span>Cassette deck · Side A / Side B</span>
            <span>Round 01</span>
          </div>
          <div className="grid items-stretch gap-3 lg:grid-cols-[1fr_84px_1fr]">
            <TapeCard
              side="a"
              track={sampleA}
              picked={false}
              playing={false}
              disabled={false}
              audioRef={audioA}
              onPlayStateChange={() => undefined}
              onPick={() => undefined}
            />
            <div className="flex items-center justify-center py-1">
              <span className="pv-vs">VS</span>
            </div>
            <TapeCard
              side="b"
              track={sampleB}
              picked={false}
              playing={false}
              disabled={false}
              audioRef={audioB}
              onPlayStateChange={() => undefined}
              onPick={() => undefined}
            />
          </div>
          <div className="mt-3 flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-paper/20 bg-paper/5 px-3 py-2.5">
            <span className="font-mono text-[9px] uppercase tracking-[0.16em] text-paper/85">
              Stamp feedback · reels spin while playing · Space / 1 / 2 / Z
            </span>
            <span className="pv-sticker px-1.5 py-0.5 text-[9px]">3 in a row</span>
          </div>
        </div>
      </Section>

      <Section number="06" title="Tracklist & podium">
        <div className="grid gap-5 lg:grid-cols-[1.2fr_0.8fr]">
          <div className="pv-card overflow-hidden">
            <div className="grid grid-cols-[2.5rem_2.5rem_1fr_auto] items-center gap-3 border-b-2 border-ink px-4 py-2.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              <span>#</span>
              <span />
              <span>Song</span>
              <span className="text-right">W · D · V · Score</span>
            </div>
            {[
              { rank: 1, title: 'Midnight City', artists: 'M83', score: 1244 },
              { rank: 2, title: 'Dreams', artists: 'Fleetwood Mac', score: 1210 },
              { rank: 3, title: 'Motion Picture Soundtrack', artists: 'Radiohead', score: 1188 },
            ].map((entry) => (
              <div
                key={entry.rank}
                className="grid grid-cols-[2.5rem_2.5rem_1fr_auto] items-center gap-3 border-b border-line px-4 py-3 last:border-b-0"
              >
                <span className="font-serif text-xl leading-none text-muted">
                  {String(entry.rank).padStart(2, '0')}
                </span>
                <span className="grid size-10 place-items-center rounded-lg border-2 border-ink bg-paper font-serif text-muted/40">
                  ♪
                </span>
                <span className="min-w-0">
                  <span className="block truncate font-semibold">{entry.title}</span>
                  <span className="block truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                    {entry.artists}
                  </span>
                </span>
                <span className="font-mono text-[9px] uppercase tracking-[0.1em] text-muted">
                  8W · 12D · 9V · <span className="text-ink">{entry.score}</span>
                </span>
              </div>
            ))}
          </div>

          <div className="pv-card p-5">
            <p className="pv-label">Top 3 podium</p>
            <div className="mt-5 flex items-end justify-center gap-2">
              {[
                { rank: 2, height: 'h-20', color: 'var(--color-cobalt)' },
                { rank: 1, height: 'h-28', color: 'var(--playlist-accent)' },
                { rank: 3, height: 'h-16', color: 'var(--color-butter)' },
              ].map((step) => (
                <div key={step.rank} className="flex w-16 flex-col items-center gap-2">
                  <span className="grid size-10 place-items-center rounded-lg border-2 border-ink bg-paper font-serif text-muted/40">
                    ♪
                  </span>
                  <div
                    className={`flex w-full ${step.height} items-start justify-center rounded-t-lg border-2 border-ink pt-2 font-serif text-2xl`}
                    style={{ background: step.color }}
                  >
                    {step.rank}
                  </div>
                </div>
              ))}
            </div>
            <p className="mt-4 text-sm leading-relaxed text-muted">
              The podium only appears on the unfiltered first page. Search results
              go straight to the tracklist.
            </p>
          </div>
        </div>
      </Section>

      <Section number="07" title="Mission control">
        <div className="rounded-3xl border-2 border-line bg-card p-5">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b-2 border-line pb-3">
            <div className="flex items-center gap-2">
              <BrandMark className="size-8" />
              <span className="font-serif text-xl text-ink">Mission control</span>
            </div>
            <span className="flex items-center gap-2 font-mono text-[9px] uppercase tracking-[0.16em] text-muted">
              <span className="size-2 rounded-full bg-ok" />
              All systems go
            </span>
          </div>
          <div className="mt-4 grid gap-3 sm:grid-cols-3">
            {[
              { label: 'Picks', value: '1,204' },
              { label: 'Voters', value: '38' },
              { label: 'Songs covered', value: '612 / 2,000' },
            ].map((stat) => (
              <div key={stat.label} className="pv-admin-stat">
                <p className="pv-admin-label">{stat.label}</p>
                <p className="mt-2 font-serif text-3xl text-ink">{stat.value}</p>
              </div>
            ))}
          </div>
          <div className="mt-3 pv-admin-panel">
            <div className="flex items-center justify-between">
              <span className="pv-admin-label">Daily picks</span>
              <span className="font-mono text-[9px] text-muted">Last 14 days</span>
            </div>
            <div className="mt-3 flex h-16 items-end gap-1">
              {[22, 38, 30, 54, 46, 70, 62, 88, 74, 56, 92, 68, 80, 96].map((height, index) => (
                <span
                  key={index}
                  className="flex-1 rounded-t-sm bg-[var(--color-cobalt)]"
                  style={{ height: `${height}%` }}
                />
              ))}
            </div>
          </div>
        </div>
      </Section>

      <p className="mt-8 border-t-2 border-ink/15 pt-4 font-mono text-[9px] uppercase tracking-[0.16em] text-muted">
        The redesign now covers home, duel, standings, Your Ranking, auth and
        mission control. This style tile is temporary and will be removed before
        launch.
      </p>
    </main>
  )
}
