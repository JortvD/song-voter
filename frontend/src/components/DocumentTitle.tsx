import { useEffect } from 'react'
import { useLocation } from 'react-router-dom'

import { usePlaylist } from '../context/PlaylistContext'

const SCREEN_TITLES: Record<string, string> = {
  '/': 'Home',
  '/duel': 'Duel',
  '/your-duels': 'Your duels',
  '/your-ranking': 'Your Ranking',
  '/standings': 'Standings',
  '/stats': 'Stats',
  '/admin': 'Admin',
  '/design': 'Design',
}

/** Keep the browser tab title in sync with the current screen and playlist. */
export function DocumentTitle() {
  const { pathname } = useLocation()
  const { status } = usePlaylist()
  const screen = SCREEN_TITLES[pathname] ?? 'Home'
  const playlist = status?.playlist_name?.trim()

  useEffect(() => {
    document.title = playlist
      ? `${screen} · ${playlist} — Playlist Voter`
      : `${screen} — Playlist Voter`
  }, [playlist, screen])

  return null
}
