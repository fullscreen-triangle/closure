/**
 * Reading the `Speaker` union.
 *
 * Every component routes through these three functions rather than inlining
 * `s === 'Player' ? … : 'Voice' in s ? …`. That is what made adding the world
 * as a speaker a change to one file instead of six.
 *
 * The kinds are not a hierarchy the UI should rank. `Voice → Agent → Player`
 * is three stages of being someone; the world sits outside that ordering
 * entirely — the city is not someone, never becomes an agent, and is never
 * pruned. It is a speaker only in the sense that a post has to come from
 * somewhere.
 */

import type { Speaker } from './api'

export type SpeakerKind = 'player' | 'voice' | 'agent' | 'world'

export function speakerKind(s: Speaker): SpeakerKind {
  if (s === 'Player') return 'player'
  if ('Voice' in s) return 'voice'
  if ('Agent' in s) return 'agent'
  return 'world'
}

/** How a speaker is named in the feed. */
export function speakerLabel(s: Speaker): string {
  if (s === 'Player') return 'you'
  if ('Voice' in s) return `voice ${s.Voice}`
  if ('Agent' in s) return s.Agent
  return `the city · ${s.World}`
}

/**
 * The voice handle, if this speaker is one.
 *
 * Returns `null` rather than a falsy number, because voice `0` is a real
 * voice and `if (speakerVoice(s))` would silently skip it.
 */
export function speakerVoice(s: Speaker): number | null {
  if (s !== 'Player' && 'Voice' in s) return s.Voice
  return null
}
