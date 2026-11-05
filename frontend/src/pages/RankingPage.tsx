import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'

import { RequireAuth } from '../components/RequireAuth'
import { ApiError, api } from '../lib/api'
import type { PersonalRanking } from '../lib/types'

const PAGE_SIZE = 50

function formatNumber(value: number) {
  return value.toLocaleString()
}

function RankingScreen() {
  const [ranking, setRanking] = useState<PersonalRanking | null>(null)
  const [offset, setOffset] = useState(0)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    setLoading(true)
    setError(null)

    api
      .getPersonalRanking(offset, PAGE_SIZE)
      .then((data) => {
        if (active) setRanking(data)
      })
      .catch((caught: unknown) => {
        if (active) {
          setError(caught instanceof ApiError ? caught.message : 'Could not load your chart')
        }
      })
      .finally(() => {
        if (active) setLoading(false)
      })

    return () => {
      active = false
    }
  }, [offset])

  const first = ranking && ranking.total > 0 ? ranking.offset + 1 : 0
  const last = ranking ? Math.min(ranking.offset + ranking.limit, ranking.total) : 0

  return (
    <>
      <div className="flex flex-wrap items-end justify-between gap-4 border-b border-line pb-4">
        <div>
          <p className="label">Personal ranking</p>
          <h1 className="mt-2 font-serif text-4xl leading-none sm:text-5xl">Your chart.</h1>
          <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {ranking
              ? `${ranking.playlist_name} · ${formatNumber(ranking.votes)} picks · ${formatNumber(ranking.total)} songs ranked`
              : 'Loading your picks…'}
          </p>
          <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
            Private to you · the shared chart stays sealed until the reveal
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Link to="/chart" className="btn-ghost">
            Shared chart
          </Link>
          <Link to="/vote" className="btn">
            Keep voting
          </Link>
        </div>
      </div>

      {error && (
        <p className="mt-4 border-l-2 border-accent bg-accent/5 px-3 py-2 text-sm text-accent">
          {error}
        </p>
      )}

      {loading ? (
        <div className="mt-6 space-y-2">
          {[0, 1, 2, 3, 4].map((index) => (
            <div key={index} className="h-14 animate-pulse border border-line bg-card" />
          ))}
        </div>
      ) : ranking && ranking.total === 0 ? (
        <section className="panel mt-6 p-6 text-center sm:p-10">
          <p className="label">No picks yet</p>
          <h2 className="mt-3 font-serif text-3xl">Your chart starts with one pick.</h2>
          <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
            Vote on a few matchups and the songs you have heard will be ranked
            here from your own answers.
          </p>
          <Link to="/vote" className="btn mt-6 inline-flex">
            Start voting
          </Link>
        </section>
      ) : ranking ? (
        <>
          <div className="mt-6 border border-line bg-card">
            <div className="hidden grid-cols-[2.5rem_3rem_1fr_auto] gap-3 border-b border-line px-4 py-2.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted sm:grid">
              <span>#</span>
              <span />
              <span>Song</span>
              <span className="text-right">Wins · Picks · Elo</span>
            </div>

            <ol>
              {ranking.tracks.map((entry) => (
                <li
                  key={entry.track.id}
                  className="grid grid-cols-[2.5rem_3rem_1fr] items-center gap-3 border-b border-line px-4 py-3 last:border-b-0 sm:grid-cols-[2.5rem_3rem_1fr_auto]"
                >
                  <span className="font-serif text-xl leading-none text-muted">
                    {String(entry.rank).padStart(2, '0')}
                  </span>

                  {entry.track.album_image_url ? (
                    <img
                      src={entry.track.album_image_url}
                      alt=""
                      className="size-12 border border-line object-cover"
                    />
                  ) : (
                    <div className="grid size-12 place-items-center border border-line bg-paper font-serif text-lg text-muted/40">
                      ♪
                    </div>
                  )}

                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium leading-tight">
                      {entry.track.title}
                    </p>
                    <p className="mt-0.5 truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                      {entry.track.artists}
                    </p>
                  </div>

                  <div className="col-span-3 flex items-center justify-end gap-4 font-mono text-[10px] uppercase tracking-[0.1em] text-muted sm:col-span-1">
                    <span>
                      <span className="text-ink">{entry.wins}</span> W
                    </span>
                    <span className="hidden sm:inline">
                      <span className="text-ink">{entry.comparisons}</span> P
                    </span>
                    <span className="text-ink">{Math.round(entry.rating)}</span>
                  </div>
                </li>
              ))}
            </ol>
          </div>

          <div className="mt-5 flex items-center justify-between gap-3">
            <button
              type="button"
              onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
              disabled={offset === 0}
              className="btn-ghost"
            >
              Previous
            </button>
            <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              {first}–{last} of {formatNumber(ranking.total)}
            </span>
            <button
              type="button"
              onClick={() => setOffset(offset + PAGE_SIZE)}
              disabled={last >= ranking.total}
              className="btn-ghost"
            >
              Next
            </button>
          </div>

          <p className="mt-4 text-center font-mono text-[9px] uppercase tracking-[0.14em] text-muted/60">
            W = wins · P = matchups · Elo = your personal rating
          </p>
        </>
      ) : null}
    </>
  )
}

export function RankingPage() {
  return (
    <main className="mx-auto w-full max-w-6xl flex-1 px-5 py-10 sm:px-8 sm:py-14">
      <RequireAuth>
        <RankingScreen />
      </RequireAuth>
    </main>
  )
}
