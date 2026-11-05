import { Navigate, Route, Routes } from 'react-router-dom'

import { Header } from './components/Header'
import { AdminPage } from './pages/AdminPage'
import { ChartPage } from './pages/ChartPage'
import { HomePage } from './pages/HomePage'
import { RankingPage } from './pages/RankingPage'
import { VotePage } from './pages/VotePage'

function App() {
  return (
    <div className="flex min-h-dvh flex-col">
      <div className="h-1 bg-accent" />
      <Header />

      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route path="/vote" element={<VotePage />} />
        <Route path="/ranking" element={<RankingPage />} />
        <Route path="/chart" element={<ChartPage />} />
        <Route path="/admin" element={<AdminPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>

      <footer className="mt-auto border-t border-ink/15">
        <div className="mx-auto flex w-full max-w-6xl flex-col gap-2 px-5 py-6 font-mono text-[9px] uppercase tracking-[0.18em] text-muted sm:flex-row sm:items-center sm:justify-between sm:px-8">
          <span>ULL Top 2000</span>
          <span>One pick at a time.</span>
        </div>
      </footer>
    </div>
  )
}

export default App
