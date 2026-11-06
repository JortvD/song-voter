import { Link } from 'react-router-dom'

type Props = {
  error: string
  onRetry: () => void | Promise<void>
  title?: string
}

/** A failed season-status request must not look like an empty season. */
export function SeasonUnavailable({
  error,
  onRetry,
  title = 'The season is unavailable.',
}: Props) {
  return (
    <section
      role="alert"
      className="pv-card mx-auto max-w-2xl p-6 text-center sm:p-10"
    >
      <span className="pv-sticker pv-sticker-paper">Connection issue</span>
      <h1 className="mt-4 font-serif text-3xl leading-tight tracking-[-0.03em] sm:text-4xl">
        {title}
      </h1>
      <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
        {error}
      </p>
      <div className="mt-6 flex flex-wrap justify-center gap-3">
        <button
          type="button"
          onClick={() => void onRetry()}
          className="btn"
        >
          Try again
        </button>
        <Link to="/" className="btn-ghost">
          Back home
        </Link>
      </div>
    </section>
  )
}
