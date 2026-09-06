/**
 * The city as 31 cells.
 *
 * Regions overlap, and the overlap is the point — it is why a voice heard in
 * one region can be audible to the character of another, and therefore why a
 * character can be reassembled at all. A list of region names hides that; a
 * strip shows it.
 */
export function PositionStrip({
  span,
  order = 31,
  mark,
}: {
  /** Positions in this region. */
  span: number[]
  order?: number
  /** One position to call out, if any. */
  mark?: number | null
}) {
  const inSpan = new Set(span)
  return (
    <span
      className="inline-flex gap-px align-middle"
      role="img"
      aria-label={`positions ${span.join(', ')}`}
    >
      {Array.from({ length: order }, (_, i) => {
        const here = inSpan.has(i)
        const called = mark === i
        return (
          <span
            key={i}
            className={
              called
                ? 'h-3 w-1 bg-[var(--ink)]'
                : here
                  ? 'h-3 w-1 bg-[var(--muted)]'
                  : 'h-3 w-1 bg-[var(--rule)]'
            }
          />
        )
      })}
    </span>
  )
}
