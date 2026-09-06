import { afterEach, describe, expect, it, vi } from 'vitest'
import { api, ApiError, normaliseToken } from './api'

function stub(status: number, body: unknown) {
  const fetchMock = vi.fn(() =>
    Promise.resolve({
      ok: status >= 200 && status < 300,
      status,
      statusText: 'stubbed',
      json: () => Promise.resolve(body),
    } as unknown as Response),
  )
  vi.stubGlobal('fetch', fetchMock)
  return fetchMock
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('normaliseToken', () => {
  it('reassembles a token retyped without dashes', () => {
    expect(normaliseToken('closureabcd2345wxyz')).toBe('CLOSURE-ABCD-2345-WXYZ')
    expect(normaliseToken('  ABCD-2345-WXYZ ')).toBe('CLOSURE-ABCD-2345-WXYZ')
  })

  it('passes anything of the wrong length straight through', () => {
    // The branch that matters: a truncated paste must reach the server as
    // typed and be refused there, rather than be silently padded into a
    // different, possibly valid, token.
    expect(normaliseToken('abcd')).toBe('ABCD')
    expect(normaliseToken('CLOSURE-ABCD-2345-WXYZ-EXTRA')).toBe(
      'CLOSURE-ABCD-2345-WXYZ-EXTRA',
    )
  })
})

describe('request', () => {
  it('turns a refusal into an ApiError carrying the server sentence', async () => {
    stub(400, { error: 'that voice has not said enough for anyone to be behind it' })
    await expect(api.character('CLOSURE-ABCD-2345-WXYZ', 3)).rejects.toMatchObject({
      status: 400,
      message: 'that voice has not said enough for anyone to be behind it',
    })
  })

  it('falls back to the status text when there is no error body', async () => {
    stub(500, {})
    const e = await api.session('T').catch((x: unknown) => x)
    expect(e).toBeInstanceOf(ApiError)
    expect((e as ApiError).status).toBe(500)
  })
})

describe('bodies', () => {
  it('sends no body with tick or prune', async () => {
    // Both endpoints have no `Json<T>` extractor. Sending `{}` "to be safe"
    // would misrepresent an endpoint whose whole point is that it takes no
    // profile.
    const f = stub(200, { tick: 1, posts: 2, weather: null })
    await api.tick('T')
    await api.prune('T', 0)
    for (const call of f.mock.calls) {
      const init = (call as unknown as [string, RequestInit | undefined])[1]
      expect(init?.method).toBe('POST')
      expect(init?.body).toBeUndefined()
    }
  })

  it('strips nulls rather than sending them as absent-but-present fields', async () => {
    const f = stub(200, {})
    await api.createPost('T', {
      parent: null,
      agent: null,
      body: 'the hydrofoil timetable is wrong',
      terminus: 7,
      acting: null,
      receiving: null,
    })
    const init = (f.mock.calls[0] as unknown as [string, RequestInit])[1]
    const body = init.body
    expect(typeof body).toBe('string')
    const sent = JSON.parse(body as string) as Record<string, unknown>
    expect(sent).toEqual({ body: 'the hydrofoil timetable is wrong', terminus: 7 })
  })
})
