import { useCallback, useEffect, useState } from 'react'
import { api, type AgentView, type AskedView, type CharacterView } from '../lib/api'
import { useAsync } from '../lib/hooks'
import { PositionStrip } from './PositionStrip'

/**
 * Voice → character → prune → ask.
 *
 * You arrive here by pointing at a post, never from a list. There is no
 * "browse all voices" anywhere in this client, because a browsable set of
 * people is a catalogue, and choosing from a catalogue is the retrieval
 * operation Theorem 4.3 says no operation has the signature of.
 *
 * A refusal is rendered as a statement about the voice, in the ordinary body
 * register — no red, no error icon, no retry button. The substrate declining
 * to build a character is the system working, not failing.
 */

/** Questions that come ready to hand. The free form is what matters. */
const PRESETS: { key: string; options: string[] }[] = [
  { key: 'grundschule', options: ['Kreis 1', 'Kreis 4', 'Kreis 6', 'Wollishofen'] },
  { key: 'district', options: ['Altstadt', 'Aussersihl', 'Oerlikon', 'Enge'] },
  { key: 'car', options: ['none', 'one', 'shared'] },
  { key: 'commute', options: ['tram', 'bicycle', 'boat', 'on foot'] },
]

export function VoicePanel({
  token,
  id,
  order,
  onBack,
  onBusy,
}: {
  token: string
  id: number
  /** Positions in this session's city. Generated, so never assumed. */
  order: number
  onBack: () => void
  /** Held true while the player is here, so the square does not move under them. */
  onBusy: (busy: boolean) => void
}) {
  const [ch, runCh] = useAsync<CharacterView>()
  const [agent, runPrune] = useAsync<AgentView>()
  const [asked, runAsk] = useAsync<AskedView>()
  const [answers, setAnswers] = useState<AskedView[]>([])
  const [key, setKey] = useState('')
  const [options, setOptions] = useState('')

  useEffect(() => {
    void runCh(() => api.character(token, id))
  }, [runCh, token, id])

  useEffect(() => {
    onBusy(true)
    return () => {
      onBusy(false)
    }
  }, [onBusy])

  const ask = useCallback(
    async (k: string, opts: string[]) => {
      await runAsk(async () => {
        const a = await api.ask(token, id, k, opts)
        setAnswers((prev) => [...prev.filter((p) => p.key !== a.key), a])
        return a
      })
    },
    [runAsk, token, id],
  )

  const freeOptions = options
    .split(',')
    .map((s) => s.trim())
    .filter((s) => s.length > 0)

  return (
    <section className="mt-10">
      <button
        type="button"
        onClick={onBack}
        className="font-mono text-xs lowercase text-[var(--muted)] underline-offset-4 hover:text-[var(--ink)] hover:underline"
      >
        ← back to the square
      </button>

      <h2 className="mt-6 font-mono text-xs uppercase tracking-[0.2em] text-[var(--muted)]">
        voice {id}
      </h2>

      {ch.kind === 'loading' && (
        <p className="mt-4 text-sm text-[var(--muted)]">reassembling…</p>
      )}

      {/* The refusal. Body register, `status` not `alert`, and no retry: the
          answer would not change, and offering one implies it might. */}
      {ch.kind === 'failed' && (
        <div className="mt-4 max-w-prose leading-relaxed" role="status">
          {ch.status === 400 ? (
            <>
              <p>
                There is nobody behind this voice yet. It has not spoken across enough
                of the city for the characters that heard it to overlap, and nothing is
                invented to cover the shortfall.
              </p>
              <p className="mt-3 text-sm text-[var(--muted)]">
                It may range further. The square is still moving.
              </p>
            </>
          ) : (
            <p className="text-sm text-[var(--muted)]">{ch.message}</p>
          )}
        </div>
      )}

      {ch.kind === 'ok' && (
        <div className="mt-4">
          <dl className="grid grid-cols-[10rem_1fr] gap-y-2 text-sm">
            <dt className="text-[var(--muted)]">heard by</dt>
            <dd>{ch.value.audible_to.join(', ') || '—'}</dd>

            <dt className="text-[var(--muted)]">spoke at</dt>
            <dd className="flex items-center gap-3">
              <PositionStrip span={ch.value.footprint} order={order} />
              <span className="font-mono text-xs text-[var(--muted)]">
                {ch.value.footprint.join(' ')}
              </span>
            </dd>

            <dt className="text-[var(--muted)]">chi</dt>
            <dd className="font-mono">
              {ch.value.chi === null ? '—' : ch.value.chi.toFixed(4)}
            </dd>
          </dl>
          {/* Never a bar, never sorted, never two voices side by side. */}
          <p className="mt-2 text-xs text-[var(--muted)]">
            chi is not a rating, and not comparable between voices.
          </p>

          <div className="mt-6 border-t border-[var(--rule)] pt-4">
            {agent.kind === 'ok' ? (
              <p className="text-sm">
                <span className="font-mono">{agent.value.id}</span> is someone now, over{' '}
                {agent.value.order} positions. Pruning is irreversible after contact, so
                this does not undo.
              </p>
            ) : (
              <>
                <button
                  type="button"
                  disabled={agent.kind === 'loading'}
                  onClick={() => {
                    void runPrune(() => api.prune(token, id))
                  }}
                  className="border border-[var(--ink)] px-3 py-2 text-sm lowercase disabled:opacity-50"
                >
                  {agent.kind === 'loading' ? 'pruning…' : 'make them someone'}
                </button>
                <p className="mt-2 text-xs text-[var(--muted)]">
                  No profile to pick. The character is built from where the voice spoke,
                  not chosen from a set.
                </p>
                {agent.kind === 'failed' && (
                  <p className="mt-2 text-sm" role="status">
                    {agent.message}
                  </p>
                )}
              </>
            )}
          </div>

          <div className="mt-6 border-t border-[var(--rule)] pt-4">
            <h3 className="font-mono text-xs uppercase tracking-[0.2em] text-[var(--muted)]">
              ask
            </h3>
            {/* What earlier questions settled, as the server holds it. Shown
                because an answer that survived the request is a fact about
                this character now, not a log of what this tab happened to
                ask. */}
            {Object.keys(ch.value.identity).length > 0 && answers.length === 0 && (
              <dl className="mt-3 grid grid-cols-[10rem_1fr] gap-y-2 text-sm">
                {Object.entries(ch.value.identity).map(([k, v]) => (
                  <div key={k} className="contents">
                    <dt className="font-mono text-[var(--muted)]">{k}</dt>
                    <dd>{v}</dd>
                  </div>
                ))}
              </dl>
            )}
            <p className="mt-2 max-w-prose text-xs leading-relaxed text-[var(--muted)]">
              Nothing is true of this character until something asks. The answer is
              generated against the options <em>you</em> give, and was not sitting
              anywhere waiting to be looked up.
            </p>

            <div className="mt-3 flex flex-wrap gap-2">
              {PRESETS.map((p) => (
                <button
                  key={p.key}
                  type="button"
                  onClick={() => void ask(p.key, p.options)}
                  className="border border-[var(--rule)] px-2 py-1 font-mono text-xs lowercase text-[var(--muted)] hover:text-[var(--ink)]"
                >
                  {p.key}
                </button>
              ))}
            </div>

            <form
              className="mt-4 flex flex-wrap items-end gap-2"
              onSubmit={(e) => {
                e.preventDefault()
                if (key.trim() && freeOptions.length > 0) {
                  void ask(key.trim(), freeOptions)
                }
              }}
            >
              <label className="flex flex-col gap-1 text-xs text-[var(--muted)]">
                question
                <input
                  value={key}
                  onChange={(e) => {
                    setKey(e.target.value)
                  }}
                  placeholder="boat"
                  className="border border-[var(--rule)] bg-transparent px-2 py-1 font-mono text-sm"
                />
              </label>
              <label className="flex flex-col gap-1 text-xs text-[var(--muted)]">
                answers it admits (comma separated)
                <input
                  value={options}
                  onChange={(e) => {
                    setOptions(e.target.value)
                  }}
                  placeholder="never, sometimes, daily"
                  className="w-72 border border-[var(--rule)] bg-transparent px-2 py-1 font-mono text-sm"
                />
              </label>
              {/* At least one option, enforced here, so the server's 400 —
                  "a question with no admissible answers has none" — is a
                  statement about the theory rather than a form error. */}
              <button
                type="submit"
                disabled={!key.trim() || freeOptions.length === 0}
                className="border border-[var(--ink)] px-3 py-1 text-sm lowercase disabled:opacity-50"
              >
                ask
              </button>
            </form>

            {asked.kind === 'failed' && (
              <p className="mt-3 text-sm" role="status">
                {asked.message}
              </p>
            )}

            {answers.length > 0 && (
              <dl className="mt-4 grid grid-cols-[10rem_1fr] gap-y-2 text-sm">
                {answers.map((a) => (
                  <div key={a.key} className="contents">
                    <dt className="font-mono text-[var(--muted)]">{a.key}</dt>
                    <dd>{a.value}</dd>
                  </div>
                ))}
              </dl>
            )}
            {answers.length > 0 && (
              // Never "1 of N": there is no N, and a fraction would imply a
              // profile that could be completed.
              <p className="mt-2 text-xs text-[var(--muted)]">
                {answers.length === 1
                  ? 'one thing is settled about them'
                  : `${answers.length} things are settled about them`}
                . There is no list of the rest.
              </p>
            )}
          </div>
        </div>
      )}
    </section>
  )
}
