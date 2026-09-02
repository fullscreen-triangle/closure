/**
 * Typed client for the closure API.
 *
 * The shapes here mirror `crates/closure-server/src/routes`. Note what is
 * absent: there is no `score`, `progress`, or `attribution` type, because the
 * server exposes no such endpoint. The runtime cannot compute a verdict
 * (Theorem 11.5) and cannot attribute a change to an action (Theorem 12.9),
 * so a client type for either would be a promise the system cannot keep.
 */

const BASE = import.meta.env.VITE_CLOSURE_API ?? ''

export interface City {
  id: string
  name: string
  substrate: string
  floor: number
}

export interface Invariant {
  index: number
  name: string
  predicate: string
  certified_by: string
}

/** What the API reports about a session: what propagated, not how it went. */
export interface SessionView {
  city: string
  /** Nodes in the world. */
  nodes: number
  /** Total emissions. Monotone; never decremented (Invariant 2). */
  record: number
  /** Stable hash of the node set; reproducibility attaches to this. */
  protocol_fingerprint: string
  /** RFC 3339. */
  opened: string
}

export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
  ) {
    super(message)
    this.name = 'ApiError'
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    ...init,
    headers: { 'content-type': 'application/json', ...init?.headers },
  })
  if (!res.ok) {
    const body = (await res.json().catch(() => ({}))) as { error?: string }
    throw new ApiError(body.error ?? res.statusText, res.status)
  }
  return (await res.json()) as T
}

export const api = {
  cities: () => request<City[]>('/v1/cities'),
  invariants: () => request<Invariant[]>('/v1/invariants'),

  openSession: (token: string, city = 'zuerich') =>
    request<{ token: string; city: string }>('/v1/session', {
      method: 'POST',
      body: JSON.stringify({ token, city }),
    }),

  session: (token: string) =>
    request<SessionView>(`/v1/session/${encodeURIComponent(token)}`),
}

/**
 * Normalise a pasted token the way the CLI does: uppercase, tolerate missing
 * dashes. A player retyping from a terminal should not be punished for either.
 */
export function normaliseToken(raw: string): string {
  const body = raw
    .trim()
    .toUpperCase()
    .replace(/^CLOSURE/, '')
    .replace(/[^0-9A-Z]/g, '')
  if (body.length !== 12) return raw.trim().toUpperCase()
  const groups = body.match(/.{1,4}/g) ?? []
  return `CLOSURE-${groups.join('-')}`
}
