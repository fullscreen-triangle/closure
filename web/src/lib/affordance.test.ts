import { describe, expect, it } from 'vitest'
import { affordanceLabel } from './affordance'

describe('affordanceLabel', () => {
  it('gives every affordance its own words', () => {
    const said = (['water', 'open', 'transit', 'indoors'] as const).map(affordanceLabel)
    expect(new Set(said).size).toBe(4)
  })

  it('says a region is unstated rather than unknown when the host is silent', () => {
    // The distinction the whole helper exists for. A host declining to
    // classify a region is not the same as the region being featureless,
    // and collapsing the two would let a reader conclude something about a
    // region from the absence of a claim about it.
    const quiet = affordanceLabel(null)
    expect(quiet).toBe('unstated')
    expect(quiet).not.toContain('unknown')
    expect(quiet).not.toContain('none')
  })

  it('carries no verdict about a region', () => {
    // Regions are not ranked. Prop. 9.6 denies a global order parameter, so
    // no label may read as better or worse than another.
    for (const a of ['water', 'open', 'transit', 'indoors', null] as const) {
      const l = affordanceLabel(a).toLowerCase()
      for (const banned of ['good', 'bad', 'best', 'poor', 'safe', 'risky']) {
        expect(l).not.toContain(banned)
      }
    }
  })
})
