import { Link } from 'react-router-dom'

import { BrandMark } from './BrandMark'
import { SoundToggle } from './SoundToggle'
import { useAuth } from '../context/AuthContext'
import { usePlaylist } from '../context/PlaylistContext'

export function Footer() {
  const { user } = useAuth()
  const { status } = usePlaylist()
  const year = new Date().getFullYear()

  return (
    <footer className="mt-auto border-t-2 border-ink/15 pb-24 lg:pb-0">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-4 px-4 py-6 sm:px-6 lg:px-8">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <Link to="/" className="flex min-w-0 items-center gap-2.5">
            <BrandMark className="size-8" />
            <span className="font-serif text-lg leading-none tracking-[-0.03em]">
              Playlist Voter
            </span>
            <span className="hidden font-mono text-[9px] uppercase tracking-[0.16em] text-muted sm:inline">
              Two songs. One pick.
            </span>
          </Link>

          <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
            {status?.playlist_name && (
              <span className="max-w-56 truncate font-mono text-[9px] uppercase tracking-[0.16em] text-muted">
                Season · {status.playlist_name}
              </span>
            )}
            <SoundToggle />
            {user?.is_admin && (
              <Link
                to="/admin"
                className="font-mono text-[9px] font-medium uppercase tracking-[0.16em] text-muted transition hover:text-ink"
              >
                Mission control
              </Link>
            )}
          </div>
        </div>

        <div className="flex flex-wrap items-center justify-between gap-2 border-t border-ink/10 pt-3 font-mono text-[9px] uppercase tracking-[0.16em] text-muted">
          <span>Made for the group chat.</span>
          <span>Playlist Voter · {year}</span>
        </div>
      </div>
    </footer>
  )
}
