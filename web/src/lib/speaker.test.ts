import { describe, expect, it } from 'vitest'
import { actLabel, actMark } from './act'
import { speakerKind, speakerLabel, speakerVoice } from './speaker'

describe('speakerKind', () => {
  it('reads all four wire forms', () => {
    expect(speakerKind('Player')).toBe('player')
    expect(speakerKind({ Voice: 3 })).toBe('voice')
    expect(speakerKind({ Agent: 'commuting/1' })).toBe('agent')
    expect(speakerKind({ World: 'open-meteo' })).toBe('world')
  })

  it('treats voice zero as a voice', () => {
    // The easy bug: `if (speakerVoice(s))` skips voice 0 silently, and voice
    // 0 is an ordinary voice.
    expect(speakerKind({ Voice: 0 })).toBe('voice')
    expect(speakerVoice({ Voice: 0 })).toBe(0)
    expect(speakerVoice('Player')).toBeNull()
    expect(speakerVoice({ World: 'open-meteo' })).toBeNull()
  })

  it('names the world by its source', () => {
    expect(speakerLabel({ World: 'open-meteo' })).toContain('open-meteo')
  })
})

describe('actMark', () => {
  it('distinguishes unmeasured from measured-and-neutral in the word', () => {
    // The one test here that is really about the theory. If these two ever
    // differ only in styling, a reader collapses "nobody looked" into
    // "nothing happened", which is what
    // `an_unmeasured_exchange_reports_null_not_inert` exists to prevent.
    const unmeasured = actLabel(null)
    const neither = actLabel({ report: false, question: false })
    expect(unmeasured).not.toBe(neither)
    expect(unmeasured).toBe('not measured')
    expect(neither).toBe('neither')
    expect(actMark(null).glyph).not.toBe(
      actMark({ report: false, question: false }).glyph,
    )
  })

  it('gives all four measured combinations their own mark', () => {
    const words = [
      actLabel({ report: true, question: false }),
      actLabel({ report: false, question: true }),
      actLabel({ report: true, question: true }),
      actLabel({ report: false, question: false }),
    ]
    expect(new Set(words).size).toBe(4)
  })

  it('carries no valence', () => {
    for (const a of [
      null,
      { report: true, question: false },
      { report: false, question: true },
      { report: true, question: true },
      { report: false, question: false },
    ]) {
      const m = actMark(a)
      const text = `${m.word} ${m.note}`.toLowerCase()
      for (const banned of ['good', 'bad', 'success', 'fail', 'score', 'better']) {
        expect(text).not.toContain(banned)
      }
    }
  })
})
