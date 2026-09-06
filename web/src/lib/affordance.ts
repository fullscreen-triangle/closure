import type { Affordance } from './api'

/**
 * How a region's affordance reads to a person.
 *
 * The region names are drawn per session — `moor-glassworks` and
 * `harbour-rowing` are handles, not descriptions, and a player cannot infer
 * anything from them. The affordance is the one thing that is *about* the
 * region, and it is what makes the weather legible: a reading that reached
 * three regions and not the fourth did so because of this, and nothing else.
 *
 * Deliberately a word and not an icon or a colour. A colour would be a
 * valence — some regions better than others — and the substrate does not
 * rank its regions. Prop. 9.6: no global order parameter.
 */
export function affordanceLabel(a: Affordance | null): string {
  switch (a) {
    case 'water':
      return 'on the water'
    case 'open':
      return 'out in the open'
    case 'transit':
      return 'a way through'
    case 'indoors':
      return 'under a roof'
    default:
      // Not "unknown". The host declining to say what a region is like is
      // not the same as the region being unremarkable.
      return 'unstated'
  }
}
