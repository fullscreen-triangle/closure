/**
 * Fetching, polling, and the heartbeat.
 *
 * `useAsync` returns a discriminated union rather than
 * `{ data, loading, error }`. The flat shape loses the distinction this app
 * most needs: a character the substrate **refused** is not an empty
 * character, and rendering it as "nothing here" would invent the very
 * absence the server declined to invent.
 */

import { useCallback, useEffect, useRef, useState } from 'react'
import { ApiError } from './api'

export type Async<T> =
  | { kind: 'idle' }
  | { kind: 'loading' }
  | { kind: 'ok'; value: T }
  | { kind: 'failed'; message: string; status: number | null }

function toFailure<T>(e: unknown): Extract<Async<T>, { kind: 'failed' }> {
  if (e instanceof ApiError) {
    return { kind: 'failed', message: e.message, status: e.status }
  }
  return { kind: 'failed', message: 'could not reach the host', status: null }
}

/** Run `fn` on demand, keeping the four states apart. */
export function useAsync<T>(): [
  Async<T>,
  (fn: () => Promise<T>) => Promise<void>,
  () => void,
] {
  const [state, setState] = useState<Async<T>>({ kind: 'idle' })
  const live = useRef(true)
  useEffect(() => {
    live.current = true
    return () => {
      live.current = false
    }
  }, [])

  const run = useCallback(async (fn: () => Promise<T>) => {
    setState({ kind: 'loading' })
    try {
      const value = await fn()
      if (live.current) setState({ kind: 'ok', value })
    } catch (e) {
      if (live.current) setState(toFailure<T>(e))
    }
  }, [])

  const reset = useCallback(() => {
    setState({ kind: 'idle' })
  }, [])

  return [state, run, reset]
}

/** True while the tab is visible. The square should not be polled unwatched. */
function useVisible(): boolean {
  const [visible, setVisible] = useState(
    () => typeof document === 'undefined' || !document.hidden,
  )
  useEffect(() => {
    const on = () => {
      setVisible(!document.hidden)
    }
    document.addEventListener('visibilitychange', on)
    return () => {
      document.removeEventListener('visibilitychange', on)
    }
  }, [])
  return visible
}

export interface Polled<T> {
  /** The last good value. Kept across a failure, so the feed never blanks. */
  value: T | null
  /** The most recent failure, if the last attempt failed. */
  error: string | null
  /** Whether a request is in flight. */
  busy: boolean
  /** Fetch now, out of band. */
  refresh: () => void
}

/**
 * Poll `fn` every `everyMs`.
 *
 * `setTimeout` chained after each settle, never `setInterval`: an interval
 * queues another request while the last is still out, and a slow host turns
 * into a pile-up. Backs off on failure, pauses when the document is hidden,
 * and keeps the last good value visible throughout.
 */
export function usePolling<T>(
  fn: (() => Promise<T>) | null,
  everyMs: number,
): Polled<T> {
  const [value, setValue] = useState<T | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [nonce, setNonce] = useState(0)
  const visible = useVisible()
  const held = useRef(fn)
  held.current = fn

  useEffect(() => {
    if (!held.current || !visible) return
    let live = true
    let timer: ReturnType<typeof setTimeout> | undefined
    let backoff = everyMs

    const cycle = async () => {
      const f = held.current
      if (!f || !live) return
      setBusy(true)
      try {
        const v = await f()
        if (!live) return
        setValue(v)
        setError(null)
        backoff = everyMs
      } catch (e) {
        if (!live) return
        setError(e instanceof ApiError ? e.message : 'could not reach the host')
        // Back off rather than hammer a host that is down, to a ceiling.
        backoff = Math.min(backoff * 2, 30_000)
      } finally {
        if (live) setBusy(false)
      }
      if (live) timer = setTimeout(() => void cycle(), backoff)
    }

    void cycle()
    return () => {
      live = false
      if (timer !== undefined) clearTimeout(timer)
    }
  }, [everyMs, visible, nonce])

  const refresh = useCallback(() => {
    setNonce((n) => n + 1)
  }, [])

  return { value, error, busy, refresh }
}

/** Whether the reader has asked for less motion. */
export function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return false
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches
  })
  useEffect(() => {
    if (!window.matchMedia) return
    const q = window.matchMedia('(prefers-reduced-motion: reduce)')
    const on = () => {
      setReduced(q.matches)
    }
    q.addEventListener('change', on)
    return () => {
      q.removeEventListener('change', on)
    }
  }, [])
  return reduced
}

export interface Heartbeat {
  running: boolean
  setRunning: (v: boolean) => void
  /** Take exactly one step. */
  step: () => void
  busy: boolean
  error: string | null
}

/**
 * The auto-tick. **A client policy, and labelled as one.**
 *
 * Nothing on the server runs on a timer: the world moves when a client asks
 * it to, which is what makes a run a reproducible sequence of requests. This
 * hook is a client deciding to ask repeatedly, and a player can hold it.
 *
 * Ticks are serialised — a step never overlaps a step — and the beat is held
 * while the tab is hidden, while `hold` is true (the player is composing or
 * asking), and by default when the reader has asked for reduced motion,
 * since the moving square *is* the motion.
 */
export function useHeartbeat(
  tick: (() => Promise<unknown>) | null,
  everyMs: number,
  hold: boolean,
): Heartbeat {
  const reduced = useReducedMotion()
  const [running, setRunning] = useState(!reduced)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [nonce, setNonce] = useState(0)
  const visible = useVisible()
  const held = useRef(tick)
  held.current = tick
  const inFlight = useRef(false)

  const once = useCallback(async () => {
    const f = held.current
    if (!f || inFlight.current) return
    inFlight.current = true
    setBusy(true)
    try {
      await f()
      setError(null)
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'could not reach the host')
    } finally {
      inFlight.current = false
      setBusy(false)
    }
  }, [])

  useEffect(() => {
    if (!running || hold || !visible || !held.current) return
    let live = true
    let timer: ReturnType<typeof setTimeout> | undefined
    const cycle = async () => {
      if (!live) return
      await once()
      if (live) timer = setTimeout(() => void cycle(), everyMs)
    }
    timer = setTimeout(() => void cycle(), everyMs)
    return () => {
      live = false
      if (timer !== undefined) clearTimeout(timer)
    }
  }, [running, hold, visible, everyMs, once, nonce])

  const step = useCallback(() => {
    void once()
    setNonce((n) => n + 1)
  }, [once])

  return { running, setRunning, step, busy, error }
}
