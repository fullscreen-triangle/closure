import { useMemo, useState } from 'react'
import type { NewPost, SubgroupView } from '../lib/api'
import { affordanceLabel } from '../lib/affordance'
import { PositionStrip } from './PositionStrip'

/**
 * Saying something, somewhere.
 *
 * The terminus is not metadata — it is the whole of what determines who can
 * hear you. So it is asked for as a place you are standing, in two steps
 * (region, then a spot inside it), with the overlaps named: standing at a
 * position two regions share means two characters hear you.
 *
 * `acting` and `receiving` are deliberately **not** exposed. They are
 * measurements of gaps before and after, not intentions a speaker declares,
 * and a field where a player typed what they meant to do would be exactly the
 * fifth operation the substrate does not have. A player's post therefore
 * comes back unmeasured, and the form says so rather than leaving it to be
 * discovered as an absence.
 */
export function Composer({
  subgroups,
  order,
  onPost,
  busy,
  error,
  onFocusChange,
}: {
  subgroups: SubgroupView[]
  /** Positions in this session's city. Generated, so never assumed. */
  order: number
  onPost: (p: NewPost) => void
  busy: boolean
  error: string | null
  onFocusChange: (composing: boolean) => void
}) {
  const [region, setRegion] = useState<string | null>(null)
  const [terminus, setTerminus] = useState<number | null>(null)
  const [body, setBody] = useState('')

  const group = subgroups.find((g) => g.name === region) ?? null

  // Regions overlap where the city does. Standing on a shared position means
  // more than one character hears you, which is worth saying out loud.
  const alsoHeard = useMemo(() => {
    if (terminus === null) return []
    return subgroups
      .filter((g) => g.name !== region && g.positions.includes(terminus))
      .map((g) => g.name)
  }, [subgroups, region, terminus])

  const ready = body.trim().length > 0 && terminus !== null

  return (
    <section className="mt-10 border-t border-[var(--rule)] pt-6">
      <h2 className="font-mono text-xs uppercase tracking-[0.2em] text-[var(--muted)]">
        say something
      </h2>

      <p className="mt-2 max-w-prose text-xs leading-relaxed text-[var(--muted)]">
        Where you stand is the whole of who hears you. Nothing else about a post decides
        its reach.
      </p>

      <div className="mt-4 flex flex-wrap gap-2">
        {subgroups.map((g) => (
          <button
            key={g.name}
            type="button"
            aria-pressed={region === g.name}
            onClick={() => {
              setRegion(g.name)
              // The midpoint, so a player who does not care still stands
              // somewhere sensible rather than on a boundary.
              const mid = g.positions[Math.floor(g.positions.length / 2)]
              setTerminus(mid ?? null)
            }}
            className={
              region === g.name
                ? 'border border-[var(--ink)] px-2 py-1 font-mono text-xs lowercase'
                : 'border border-[var(--rule)] px-2 py-1 font-mono text-xs lowercase text-[var(--muted)] hover:text-[var(--ink)]'
            }
          >
            {g.name}
          </button>
        ))}
      </div>

      {group && (
        <div className="mt-4">
          <div className="flex flex-wrap items-center gap-3">
            <PositionStrip span={group.positions} order={order} mark={terminus} />
            <span className="font-mono text-xs text-[var(--muted)]">
              standing at {terminus ?? '—'} · {affordanceLabel(group.affords)}
            </span>
          </div>
          <div className="mt-2 flex flex-wrap gap-1">
            {group.positions.map((p) => (
              <button
                key={p}
                type="button"
                aria-pressed={terminus === p}
                onClick={() => {
                  setTerminus(p)
                }}
                className={
                  terminus === p
                    ? 'border border-[var(--ink)] px-2 py-0.5 font-mono text-xs'
                    : 'border border-[var(--rule)] px-2 py-0.5 font-mono text-xs text-[var(--muted)] hover:text-[var(--ink)]'
                }
              >
                {p}
              </button>
            ))}
          </div>
          {alsoHeard.length > 0 && (
            <p className="mt-2 text-xs text-[var(--muted)]">
              also heard in {alsoHeard.join(', ')}
            </p>
          )}
        </div>
      )}

      <form
        className="mt-4"
        onSubmit={(e) => {
          e.preventDefault()
          if (!ready || terminus === null) return
          onPost({
            parent: null,
            agent: null,
            body: body.trim(),
            terminus,
            acting: null,
            receiving: null,
          })
          setBody('')
        }}
      >
        <textarea
          value={body}
          rows={3}
          onFocus={() => {
            onFocusChange(true)
          }}
          onBlur={() => {
            onFocusChange(false)
          }}
          onChange={(e) => {
            setBody(e.target.value)
          }}
          placeholder="the hydrofoil timetable is wrong"
          className="w-full resize-y border border-[var(--rule)] bg-transparent px-3 py-2 leading-relaxed"
        />
        <div className="mt-2 flex flex-wrap items-center gap-4">
          <button
            type="submit"
            disabled={!ready || busy}
            className="border border-[var(--ink)] px-3 py-2 text-sm lowercase disabled:opacity-50"
          >
            {busy ? 'registering…' : 'register it'}
          </button>
          <p className="text-xs text-[var(--muted)]">
            Your post comes back unmeasured. Nobody measured the gaps on either side of
            it, and an unmeasured act is not an inert one.
          </p>
        </div>
        {error && (
          <p className="mt-2 text-sm" role="status">
            {error}
          </p>
        )}
      </form>
    </section>
  )
}
