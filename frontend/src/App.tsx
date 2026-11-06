import { Suspense, lazy, useState } from 'react'
import { Navigate, Route, Routes } from 'react-router-dom'

import { BottomTabs } from './components/BottomTabs'
import { DocumentTitle } from './components/DocumentTitle'
import { Footer } from './components/Footer'
import { Header } from './components/Header'
import { usePlaylist } from './context/PlaylistContext'
import { HomePage } from './pages/HomePage'
import { VotePage } from './pages/VotePage'

const AdminPage = lazy(() =>
  import('./pages/AdminPage').then((module) => ({ default: module.AdminPage })),
)
const ChartPage = lazy(() =>
  import('./pages/ChartPage').then((module) => ({ default: module.ChartPage })),
)
const DesignPage = lazy(() =>
  import('./pages/DesignPage').then((module) => ({ default: module.DesignPage })),
)
const DuelHistoryPage = lazy(() =>
  import('./pages/DuelHistoryPage').then((module) => ({
    default: module.DuelHistoryPage,
  })),
)
const RankingPage = lazy(() =>
  import('./pages/RankingPage').then((module) => ({
    default: module.RankingPage,
  })),
)
const StatsPage = lazy(() =>
  import('./pages/StatsPage').then((module) => ({ default: module.StatsPage })),
)

function PageFallback() {
  return (
    <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-6 sm:px-6 sm:py-8 lg:px-8">
      <div className="h-28 animate-pulse rounded-2xl border-2 border-line bg-card" />
      <div className="mt-6 space-y-2">
        {[0, 1, 2, 3, 4, 5].map((index) => (
          <div
            key={index}
            className="h-16 animate-pulse rounded-xl border-2 border-line bg-card"
          />
        ))}
      </div>
    </main>
  )
}

function PlaylistErrorBanner() {
  const { error, refresh, status } = usePlaylist()
  const [retrying, setRetrying] = useState(false)
  const hasKnownSeason = Boolean(status?.playlist_id)

  if (!error) return null

  async function retry() {
    setRetrying(true)
    try {
      await refresh()
    } finally {
      setRetrying(false)
    }
  }

  return (
    <div role="alert" className="border-b-2 border-danger/30 bg-danger/5">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-2 px-4 py-2 sm:flex-row sm:items-center sm:justify-between sm:px-6 lg:px-8">
        <p className="min-w-0 break-words text-xs leading-relaxed text-danger">
          <strong className="font-semibold">
            {hasKnownSeason
              ? 'Could not refresh the current season.'
              : 'Could not load the current season.'}
          </strong>{' '}
          <span className="text-danger/80">{error}</span>
          {hasKnownSeason ? ' Showing the last known data.' : ''}
        </p>
        <button
          type="button"
          onClick={() => void retry()}
          disabled={retrying}
          className="shrink-0 rounded-full border-2 border-danger bg-card px-3 py-1.5 font-mono text-[9px] font-semibold uppercase tracking-[0.12em] text-danger transition hover:bg-danger hover:text-paper disabled:cursor-not-allowed disabled:opacity-40"
        >
          {retrying ? 'Retrying…' : 'Retry'}
        </button>
      </div>
    </div>
  )
}

function App() {
  return (
    <div className="flex min-h-dvh flex-col">
      <DocumentTitle />
      <a
        href="#main"
        className="sr-only z-50 rounded-full border-2 border-ink bg-card px-4 py-2 font-mono text-[10px] uppercase tracking-[0.14em] focus:not-sr-only focus:fixed focus:left-3 focus:top-3"
      >
        Skip to content
      </a>

      <Header />
      <PlaylistErrorBanner />

      <div id="main" className="flex flex-1 flex-col">
        <Suspense fallback={<PageFallback />}>
          <Routes>
            <Route path="/" element={<HomePage />} />
            <Route path="/duel" element={<VotePage />} />
            <Route path="/your-duels" element={<DuelHistoryPage />} />
            <Route path="/your-ranking" element={<RankingPage />} />
            <Route path="/standings" element={<ChartPage />} />
            <Route path="/stats" element={<StatsPage />} />
            <Route path="/admin" element={<AdminPage />} />
            <Route path="/design" element={<DesignPage />} />
            <Route path="/vote" element={<Navigate to="/duel" replace />} />
            <Route path="/ranking" element={<Navigate to="/your-ranking" replace />} />
            <Route path="/chart" element={<Navigate to="/standings" replace />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </Suspense>
      </div>

      <Footer />
      <BottomTabs />
    </div>
  )
}

export default App
