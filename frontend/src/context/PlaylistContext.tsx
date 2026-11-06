import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import type { ReactNode } from 'react'
import { useLocation } from 'react-router-dom'

import { ApiError, api } from '../lib/api'
import type { ChartStatus } from '../lib/types'
import { applyPlaylistAccent, extractPlaylistAccent } from '../lib/playlistAccent'

type PlaylistContextValue = {
  status: ChartStatus | null
  loading: boolean
  error: string | null
  refresh: () => Promise<void>
}

const PlaylistContext = createContext<PlaylistContextValue | null>(null)

export function PlaylistProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<ChartStatus | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const location = useLocation()

  const refresh = useCallback(async () => {
    try {
      const next = await api.getChartStatus()
      setStatus(next)
      setError(null)
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : 'Could not load the current playlist',
      )
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void refresh()
  }, [location.pathname, refresh])

  useEffect(() => {
    let active = true
    const imageUrl = status?.playlist_image_url

    if (!imageUrl) {
      applyPlaylistAccent(null)
      return
    }

    void extractPlaylistAccent(imageUrl).then((accent) => {
      if (active) applyPlaylistAccent(accent)
    })

    return () => {
      active = false
    }
  }, [status?.playlist_image_url])

  const value = useMemo(
    () => ({ status, loading, error, refresh }),
    [status, loading, error, refresh],
  )

  return <PlaylistContext.Provider value={value}>{children}</PlaylistContext.Provider>
}

export function usePlaylist(): PlaylistContextValue {
  const context = useContext(PlaylistContext)

  if (!context) {
    throw new Error('usePlaylist must be used inside PlaylistProvider')
  }

  return context
}
