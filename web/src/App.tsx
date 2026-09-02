import { useCallback, useEffect, useState } from 'react'
import { api, ApiError, normaliseToken, type SessionView } from './lib/api'
import { JoinForm } from './components/JoinForm'
import { SessionPanel } from './components/SessionPanel'

/**
 * The interaction surface.
 *
 * A player arrives here with a token minted by the CLI, pastes it, and is
 * joined to a running city. There is no lobby, no difficulty select, and no
 * progress bar — the last of those is not an omission but a consequence: the
 * runtime cannot compute a verdict, so a client showing one would be
 * displaying a number the server does not have.
 */
export function App() {
  const [token, setToken] = useState<string | null>(null)
  const [session, setSession] = useState<SessionView | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  // A token may arrive in the URL, because `closure session new` opens the
  // browser for you.
  useEffect(() => {
    const fromUrl = new URLSearchParams(window.location.search).get('token')
    if (fromUrl) setToken(normaliseToken(fromUrl))
  }, [])

  const join = useCallback(async (raw: string) => {
    const t = normaliseToken(raw)
    setBusy(true)
    setError(null)
    try {
      await api.openSession(t)
      setSession(await api.session(t))
      setToken(t)
      window.history.replaceState({}, '', `?token=${encodeURIComponent(t)}`)
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'could not reach the host')
    } finally {
      setBusy(false)
    }
  }, [])

  useEffect(() => {
    if (token && !session && !busy) void join(token)
  }, [token, session, busy, join])

  return (
    <main className="mx-auto flex min-h-screen max-w-3xl flex-col px-6 py-16">
      <header className="mb-12">
        <h1 className="font-mono text-sm uppercase tracking-[0.3em] opacity-60">
          closure
        </h1>
        <p className="mt-3 text-2xl leading-snug">
          Talk to a city that is not waiting for you.
        </p>
      </header>

      {session ? (
        <SessionPanel session={session} token={token ?? ''} />
      ) : (
        <JoinForm onJoin={join} busy={busy} error={error} />
      )}

      <footer className="mt-auto pt-16 text-xs leading-relaxed opacity-50">
        <p>
          This instrument reports what propagated. It does not report whether you
          succeeded, and it cannot tell you which of your actions mattered — both are
          consequences of the theory it implements rather than missing features.
        </p>
      </footer>
    </main>
  )
}
