/**
 * Typed client for the closure API.
 *
 * The shapes here mirror `crates/closure-server/src/routes`. Note what is
 * absent: there is no `score`, `progress`, or `attribution` type, because the
 * server exposes no such endpoint. The runtime cannot compute a verdict
 * (Theorem 11.5) and cannot attribute a change to an action (Theorem 12.9),
 * so a client type for either would be a promise the system cannot keep.
 *
 * There is also no `search` and no `voices()` that takes anything but a
 * region. The only path to a voice is: read a region, point at who is
 * talking there. A convenience wrapper that swept every region into one list
 * would rebuild the catalogue Theorem 4.3 denies, on the client, which is no
 * better than doing it on the server.
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

/**
 * One reading of the city. Display only.
 *
 * Its mechanical effect is entirely in the termini its posts registered at —
 * the text here changed nothing and is not read by anything.
 */
export interface Observation {
  temperature_c: number
  wind_kph: number
  precipitation_mm: number
  code: number
  taken: string
  source: string
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
  /** Posts registered so far. A count, not a ranking. */
  posts: number
  /** The world tick. Advances only when a client asks it to. */
  tick: number
  /** Voices in the square. Not a headcount of anyone real. */
  voices: number
  /** The seed this square opened with. */
  seed: number
  /** The last reading registered here, if the world is reporting. */
  weather: Observation | null
}

/**
 * Who spoke.
 *
 * Externally tagged, and `VoiceId` is a transparent newtype, so the wire form
 * of voice three is `{"Voice":3}` — not `{"Voice":{"0":3}}`. Read this union
 * through `lib/speaker`, never by inlining the checks.
 */
export type Speaker =
  'Player' | { Voice: number } | { Agent: string } | { World: string }

/**
 * The two independent directions of Theorem 7.5.
 *
 * Independent, so all four combinations occur and none is a degenerate case.
 */
export interface ActView {
  report: boolean
  question: boolean
}

export interface PostView {
  id: number
  parent: number | null
  speaker: Speaker
  body: string
  terminus: number
  record: number
  at: number
  /**
   * Direction only. `null` means the gaps were **not measured**, which is not
   * the same as measured-and-unmoved.
   */
  act: ActView | null
}

export interface SubgroupView {
  name: string
  /** Positions this region spans. Visibility follows from these. */
  positions: number[]
  /** Voices heard here. A count of handles, not of people. */
  voices: number
}

export interface VoiceView {
  id: number
  name: string
  spoken_at: number[]
  subgroups: string[]
  posts: number
}

export interface CharacterView {
  voice: number
  audible_to: string[]
  footprint: number[]
  /** The character invariant. Not a rating, and not comparable across voices. */
  chi: number | null
  /** What has been settled about who would have this character. */
  identity: Record<string, string>
}

export interface AgentView {
  id: string
  order: number
  record: number
  chi: number | null
}

export interface AskedView {
  key: string
  value: string
  /** How many attributes exist after the question. Never a completion count. */
  settled: number
}

export interface TickView {
  tick: number
  posts: number
  weather: Observation | null
}

/**
 * A post to register.
 *
 * The four optional server fields are **explicit nullables** rather than
 * optional properties. Under `exactOptionalPropertyTypes` a `parent?: number`
 * refuses `{ parent: undefined }`, so every spread-built object would fail to
 * typecheck; and the composer needs a real "no parent" value in state
 * regardless. Nulls are stripped at the request boundary, and serde treats
 * absent and explicit `null` alike.
 */
export interface NewPost {
  parent: number | null
  agent: string | null
  body: string
  terminus: number
  /** Gaps on the acting side, before and after. */
  acting: [number, number] | null
  /** Gaps on the receiving side, before and after. */
  receiving: [number, number] | null
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

function stripNulls(v: Record<string, unknown>): Record<string, unknown> {
  return Object.fromEntries(Object.entries(v).filter(([, x]) => x !== null))
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

const t = (token: string) => encodeURIComponent(token)

export const api = {
  cities: () => request<City[]>('/v1/cities'),
  invariants: () => request<Invariant[]>('/v1/invariants'),

  openSession: (token: string, city = 'zuerich') =>
    request<{ token: string; city: string }>('/v1/session', {
      method: 'POST',
      body: JSON.stringify({ token, city }),
    }),

  session: (token: string) => request<SessionView>(`/v1/session/${t(token)}`),

  /**
   * The feed. No `order` parameter is sent: the handler has no viewer, so it
   * serves `recent` for both values. Offering a control that changes nothing
   * would be a lie about what the server does.
   */
  posts: (token: string) => request<PostView[]>(`/v1/session/${t(token)}/posts`),

  thread: (token: string, id: number) =>
    request<PostView[]>(`/v1/session/${t(token)}/thread/${id}`),

  createPost: (token: string, post: NewPost) =>
    request<PostView>(`/v1/session/${t(token)}/posts`, {
      method: 'POST',
      body: JSON.stringify(stripNulls(post as unknown as Record<string, unknown>)),
    }),

  /** Advances the world one step. Takes no body. */
  tick: (token: string) =>
    request<TickView>(`/v1/session/${t(token)}/tick`, { method: 'POST' }),

  subgroups: (token: string) =>
    request<SubgroupView[]>(`/v1/session/${t(token)}/subgroups`),

  /** Voices in one region. There is no call that takes a description. */
  voices: (token: string, region: string) =>
    request<VoiceView[]>(
      `/v1/session/${t(token)}/subgroups/${encodeURIComponent(region)}/voices`,
    ),

  character: (token: string, id: number) =>
    request<CharacterView>(`/v1/session/${t(token)}/voices/${id}/character`),

  /**
   * Makes a voice someone. Takes no body — there is no profile to name and
   * no tiebreak to declare. Sending `{}` "to be safe" would misrepresent an
   * endpoint whose whole point is that it accepts nothing.
   */
  prune: (token: string, id: number) =>
    request<AgentView>(`/v1/session/${t(token)}/voices/${id}/prune`, {
      method: 'POST',
    }),

  ask: (token: string, id: number, key: string, options: string[]) =>
    request<AskedView>(`/v1/session/${t(token)}/voices/${id}/ask`, {
      method: 'POST',
      body: JSON.stringify({ key, options }),
    }),
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
