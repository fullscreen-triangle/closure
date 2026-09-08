import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

/**
 * The theme is dark, and that is a decision rather than a preference.
 *
 * There is no DOM here, so these read the stylesheet as text. That is the
 * right granularity for the claim: what matters is not which hex values are
 * used but that the palette is *unconditional* — a reader on a light desktop
 * must get the same surface as a reader on a dark one. A regression here
 * would not throw or fail to compile, it would quietly restore a second
 * design nobody is tuning.
 */
const css = readFileSync(fileURLToPath(new URL('../index.css', import.meta.url)), 'utf8')
const html = readFileSync(fileURLToPath(new URL('../../index.html', import.meta.url)), 'utf8')

/** The body of the bare `:root` block, or '' if there is none. */
function rootBlock(): string {
  return /:root\s*\{([^}]*)\}/.exec(css)?.[1] ?? ''
}

describe('the dark theme', () => {
  it('declares the palette on bare :root, not inside a media query', () => {
    const root = rootBlock()
    expect(root, 'no :root block').not.toBe('')
    for (const token of ['--ink', '--paper', '--muted', '--rule']) {
      expect(root, `${token} must be defined unconditionally`).toContain(token)
    }
  })

  it('does not switch palettes on the reader operating system', () => {
    // The at-rule, not the word: the comment above the palette explains why
    // the media query was removed, and naming it there is not using it.
    expect(css).not.toMatch(/@media[^{]*prefers-color-scheme/)
  })

  it('commits color-scheme so form controls and scrollbars follow', () => {
    // Without this the browser paints native widgets light on a dark ground.
    expect(css).toMatch(/color-scheme:\s*dark\s*;/)
    expect(html).toMatch(/name="color-scheme"\s+content="dark"/)
  })

  it('paints a dark ground before the stylesheet lands', () => {
    // A theme-color matching --paper is what stops a white flash on load.
    expect(html).toMatch(/name="theme-color"\s+content="#14120f"/)
  })

  it('keeps metadata a distinct colour rather than a faded foreground', () => {
    // --muted exists so metadata is not rendered with opacity, which fails
    // contrast and reads as "the same text, dimmer" rather than as a
    // different register.
    const root = rootBlock()
    const ink = /--ink:\s*(#[0-9a-f]{6})/i.exec(root)?.[1]
    const muted = /--muted:\s*(#[0-9a-f]{6})/i.exec(root)?.[1]
    expect(ink).toBeDefined()
    expect(muted).toBeDefined()
    expect(muted).not.toBe(ink)
  })
})
