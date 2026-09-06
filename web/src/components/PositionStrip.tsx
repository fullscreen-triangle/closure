/**
 * The city as a strip of cells, one per position.
 *
 * Regions overlap, and the overlap is the point — it is why a voice heard in
 * one region can be audible to the character of another, and therefore why a
 * character can be reassembled at all. A list of region names hides that; a
 * strip shows it.
 */
export function PositionStrip({
  span,
  order,
  mark,
}: {
  /** Positions in this region. */
  span: number[]
  /**
   * Positions in the city. Required, and deliberately not defaulted: the
   * society is generated per session, so its width differs every time. A
   * default would render a strip that looked right and was not — cells
   * beyond it vanishing, or phantom cells past the end of the world.
   */
  order: number
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
