import { Navigate, Route, Routes } from 'react-router-dom'

import { BottomTabs } from './components/BottomTabs'
import { Footer } from './components/Footer'
import { Header } from './components/Header'
import { AdminPage } from './pages/AdminPage'
import { ChartPage } from './pages/ChartPage'
import { DesignPage } from './pages/DesignPage'
import { HomePage } from './pages/HomePage'
import { RankingPage } from './pages/RankingPage'
import { VotePage } from './pages/VotePage'

function App() {
  return (
    <div className="flex min-h-dvh flex-col">
      <a
        href="#main"
        className="sr-only z-50 rounded-full border-2 border-ink bg-card px-4 py-2 font-mono text-[10px] uppercase tracking-[0.14em] focus:not-sr-only focus:fixed focus:left-3 focus:top-3"
      >
        Skip to content
      </a>

      <Header />

      <div id="main" className="flex flex-1 flex-col">
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/duel" element={<VotePage />} />
          <Route path="/your-ranking" element={<RankingPage />} />
          <Route path="/standings" element={<ChartPage />} />
          <Route path="/admin" element={<AdminPage />} />
          <Route path="/design" element={<DesignPage />} />
          <Route path="/vote" element={<Navigate to="/duel" replace />} />
          <Route path="/ranking" element={<Navigate to="/your-ranking" replace />} />
          <Route path="/chart" element={<Navigate to="/standings" replace />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </div>

      <Footer />
      <BottomTabs />
    </div>
  )
}

export default App
