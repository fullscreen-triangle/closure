/**
 * Rendering what a post did to the gaps.
 *
 * Theorem 7.5 gives two **independent** directions, so all four combinations
 * are ordinary and none is degenerate. What this module exists to protect is
 * the fifth case: `act === null` means the gaps were not measured, which is
 * not the same as measured-and-unmoved.
 *
 * That distinction must survive in the **word**, not merely in the styling.
 * If unmeasured and neither differ only by opacity, a reader collapses them,
 * and the server test `an_unmeasured_exchange_reports_null_not_inert` exists
 * precisely to stop that collapse.
 *
 * Deliberately absent: colour semantics (green/red is valence, i.e. a
 * verdict), any count, any aggregate across a thread or a voice (Prop. 9.6
 * — there is no global order parameter to sum toward), and any sort or
 * filter by act.
 */

import type { ActView } from './api'

export interface ActMark {
  glyph: string
  word: string
  /** Long form, for a title attribute. */
  note: string
}

export function actMark(act: ActView | null): ActMark {
  if (act === null) {
    return {
      glyph: '—',
      word: 'not measured',
      note: 'the gaps were not measured; that is not the same as measured and unmoved',
    }
  }
  if (act.report && act.question) {
    return {
      glyph: '→?',
      word: 'both',
      note: 'reported and questioned at once — the two directions are independent',
    }
  }
  if (act.report) {
    return { glyph: '→', word: 'report', note: 'moved the acting gap' }
  }
  if (act.question) {
    return { glyph: '?', word: 'question', note: 'moved the receiving gap' }
  }
  return { glyph: '·', word: 'neither', note: 'measured, and moved neither gap' }
}

/** The word alone. Used where a glyph would not fit. */
export function actLabel(act: ActView | null): string {
  return actMark(act).word
}
