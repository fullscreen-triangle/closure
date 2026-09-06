import { actMark } from '../lib/act'
import type { ActView } from '../lib/api'

/**
 * What a post did to the gaps.
 *
 * Glyph *and* word, always both. An unmeasured act must differ from a
 * measured-and-neutral one in kind rather than in weight, because a reader
 * who sees only a paler version of the same mark will read "nothing
 * happened" where the truth is "nobody looked".
 */
export function ActMark({ act }: { act: ActView | null }) {
  const m = actMark(act)
  return (
    <span
      className="text-[var(--muted)] whitespace-nowrap"
      title={m.note}
      aria-label={m.word}
    >
      <span aria-hidden="true" className="font-mono">
        {m.glyph}
      </span>{' '}
      {m.word}
    </span>
  )
}
