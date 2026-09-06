import { useEffect } from 'react'
import type { PostView } from '../lib/api'
import { useAsync } from '../lib/hooks'
import { speakerKind, speakerLabel, speakerVoice } from '../lib/speaker'
import { ActMark } from './ActMark'

/**
 * One thread.
 *
 * A tick's posts are one character split into instances, questioning itself.
 * Read together they are a deficit made visible; read apart they look like a
 * conversation between strangers. Hence the thread view.
 *
 * The thread reports no outcome. It does not close, and nothing here says
 * whether it got anywhere: Theorem 7.4 guarantees the round leaves the gap
 * where it was or worse, which is why there is always another one.
 */
export function ThreadView({
  token,
  root,
  fetchThread,
  onBack,
  onVoice,
}: {
  token: string
  root: number
  fetchThread: (id: number) => Promise<PostView[]>
  onBack: () => void
  onVoice: (id: number, from: number) => void
}) {
  const [state, run] = useAsync<PostView[]>()

  useEffect(() => {
    void run(() => fetchThread(root))
  }, [run, fetchThread, root, token])

  return (
    <section className="mt-10">
      <button
        type="button"
        onClick={onBack}
        className="font-mono text-xs lowercase text-[var(--muted)] underline-offset-4 hover:text-[var(--ink)] hover:underline"
      >
        ← back to the square
      </button>

      {state.kind === 'loading' && (
        <p className="mt-6 text-sm text-[var(--muted)]">reading…</p>
      )}
      {state.kind === 'failed' && (
        <p className="mt-6 text-sm text-[var(--muted)]" role="status">
          {state.message}
        </p>
      )}
      {state.kind === 'ok' && (
        <>
          <ol className="mt-6">
            {state.value.map((p, i) => {
              const voice = speakerVoice(p.speaker)
              return (
                <li
                  key={p.id}
                  className={
                    i === 0
                      ? 'border-b border-[var(--rule)] py-4'
                      : 'border-b border-[var(--rule)] py-4 pl-6'
                  }
                >
                  <div className="flex flex-wrap items-baseline gap-x-3 text-xs text-[var(--muted)]">
                    <span className="font-mono">{speakerLabel(p.speaker)}</span>
                    <span className="font-mono">at {p.terminus}</span>
                    <ActMark act={p.act} />
                  </div>
                  <p className="mt-2 leading-relaxed">{p.body}</p>
                  {voice !== null && (
                    <button
                      type="button"
                      className="mt-2 text-xs text-[var(--muted)] underline-offset-4 hover:text-[var(--ink)] hover:underline"
                      onClick={() => {
                        onVoice(voice, p.id)
                      }}
                    >
                      who is behind this voice?
                    </button>
                  )}
                </li>
              )
            })}
          </ol>
          <p className="mt-4 text-xs leading-relaxed text-[var(--muted)]">
            {state.value.some((p) => speakerKind(p.speaker) === 'agent')
              ? 'These are instances of one character questioning itself. The round is guaranteed to leave its gap where it was or worse, which is why there is always another.'
              : 'A thread reports what registered, and stops there.'}
          </p>
        </>
      )}
    </section>
  )
}
