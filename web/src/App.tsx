import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  api,
  ApiError,
  normaliseToken,
  type NewPost,
  type PostView,
  type SessionView,
  type SubgroupView,
} from './lib/api'
import { useAsync, useHeartbeat, usePolling } from './lib/hooks'
import { JoinForm } from './components/JoinForm'
import { SessionPanel } from './components/SessionPanel'
import { WorldClock } from './components/WorldClock'
import { Feed } from './components/Feed'
import { ThreadView } from './components/ThreadView'
import { VoicePanel } from './components/VoicePanel'
import { Composer } from './components/Composer'

/**
 * The interaction surface.
 *
 * A player arrives with a token minted by the CLI, pastes it, and is joined
 * to a running city. There is no lobby, no difficulty select, and no progress
 * bar — the last is not an omission but a consequence: the runtime cannot
 * compute a verdict, so a client showing one would be displaying a number the
 * server does not have.
 *
 * **No router.** The app has one address — the token, already in `?token=` —
 * and a `/voice/7` route would make voices linkable and therefore
 * enumerable, reintroducing at the URL layer the retrieval the API refuses.
 * Switching between the square, a thread, and a voice is *focus*, not
 * navigation: the feed stays live beside whatever is open.
 */
type Focus =
  | { kind: 'square' }
  | { kind: 'thread'; root: number }
  | { kind: 'voice'; id: number; from: number }

export function App() {
  const [token, setToken] = useState<string | null>(null)
  // The city the session was opened under. Held so a reload can rejoin the
  // same square: the name seeds the draw, so rejoining under a different
  // one would silently be a different city.
  const [city, setCity] = useState<string | null>(null)
  const [joined, setJoined] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [focus, setFocus] = useState<Focus>({ kind: 'square' })
  const [region, setRegion] = useState<string | null>(null)
  const [composing, setComposing] = useState(false)
  const [reading, setReading] = useState(false)

  // A token may arrive in the URL, because `closure session new` opens the
  // browser for you. It carries the city alongside, since the pair is what
  // draws the square — a token on its own does not name a world.
  useEffect(() => {
    const q = new URLSearchParams(window.location.search)
    const fromUrl = q.get('token')
    const named = q.get('city')
    if (fromUrl) setToken(normaliseToken(fromUrl))
    if (named !== null && named.trim()) setCity(named.trim())
  }, [])

  const join = useCallback(async (raw: string, named: string) => {
    const t = normaliseToken(raw)
    const c = named.trim()
    if (!c) return
    setBusy(true)
    setError(null)
    try {
      await api.openSession(t, c)
      await api.session(t)
      setToken(t)
      setCity(c)
      setJoined(true)
      window.history.replaceState(
        {},
        '',
        `?token=${encodeURIComponent(t)}&city=${encodeURIComponent(c)}`,
      )
    } catch (e) {
      setError(e instanceof ApiError ? e.message : 'could not reach the host')
    } finally {
      setBusy(false)
    }
  }, [])

  // Rejoin without asking only when the URL supplied both halves. With a
  // token and no city there is nothing to open — the form asks rather than
  // picking a name on the player's behalf.
  useEffect(() => {
    if (token && city && !joined && !busy) void join(token, city)
  }, [token, city, joined, busy, join])

  const live = joined && token !== null ? token : null

  const session = usePolling<SessionView>(
    useMemo(() => (live ? () => api.session(live) : null), [live]),
    4000,
  )
  const posts = usePolling<PostView[]>(
    useMemo(() => (live ? () => api.posts(live) : null), [live]),
    3000,
  )
  // The regions do not change during a session, so this is fetched once and
  // then left alone.
  const [groups, runGroups] = useAsync<SubgroupView[]>()
  useEffect(() => {
    if (live) void runGroups(() => api.subgroups(live))
  }, [live, runGroups])

  const refreshBoth = posts.refresh
  const beat = useHeartbeat(
    useMemo(
      () =>
        live
          ? async () => {
              await api.tick(live)
              refreshBoth()
            }
          : null,
      [live, refreshBoth],
    ),
    8000,
    // Held while the player is composing or reading a character: the square
    // should not move out from under someone who is mid-sentence.
    composing || reading,
  )

  const [posted, runPost] = useAsync<PostView>()
  const submit = useCallback(
    (p: NewPost) => {
      if (!live) return
      void runPost(async () => {
        const out = await api.createPost(live, p)
        refreshBoth()
        return out
      })
    },
    [live, runPost, refreshBoth],
  )

  const fetchThread = useCallback(
    (id: number) => {
      if (!live) return Promise.resolve<PostView[]>([])
      return api.thread(live, id)
    },
    [live],
  )

  const subgroups = groups.kind === 'ok' ? groups.value : []

  if (!live) {
    return (
      <Shell>
        <JoinForm
          onJoin={join}
          /* A token can arrive from the CLI without a city; keep it in the
             field so the player fills in the name rather than re-pasting. */
          initialToken={token ?? ''}
          busy={busy}
          error={error}
        />
      </Shell>
    )
  }

  return (
    <Shell>
      {session.value && <SessionPanel session={session.value} token={live} />}

      <WorldClock
        tick={session.value?.tick ?? 0}
        beat={beat}
        weather={session.value?.weather ?? null}
      />

      {focus.kind === 'square' && (
        <>
          <Feed
            posts={posts.value ?? []}
            subgroups={subgroups}
            region={region}
            onRegion={setRegion}
            onThread={(root) => {
              setFocus({ kind: 'thread', root })
            }}
            onVoice={(id, from) => {
              setFocus({ kind: 'voice', id, from })
            }}
            stale={posts.error}
          />
          <Composer
            subgroups={subgroups}
            order={session.value?.order ?? 0}
            onPost={submit}
            busy={posted.kind === 'loading'}
            error={posted.kind === 'failed' ? posted.message : null}
            onFocusChange={setComposing}
          />
        </>
      )}

      {focus.kind === 'thread' && (
        <ThreadView
          token={live}
          root={focus.root}
          fetchThread={fetchThread}
          onBack={() => {
            setFocus({ kind: 'square' })
          }}
          onVoice={(id, from) => {
            setFocus({ kind: 'voice', id, from })
          }}
        />
      )}

      {focus.kind === 'voice' && (
        <VoicePanel
          token={live}
          id={focus.id}
          order={session.value?.order ?? 0}
          onBack={() => {
            setFocus({ kind: 'square' })
          }}
          onBusy={setReading}
        />
      )}
    </Shell>
  )
}

function Shell({ children }: { children: React.ReactNode }) {
  return (
    <main className="mx-auto flex min-h-screen max-w-3xl flex-col px-6 py-16">
      <header className="mb-10">
        <h1 className="font-mono text-sm uppercase tracking-[0.3em] text-[var(--muted)]">
          closure
        </h1>
        <p className="mt-3 text-2xl leading-snug">
          Talk to a city that is not waiting for you.
        </p>
      </header>

      {children}

      <footer className="mt-auto pt-16 text-xs leading-relaxed text-[var(--muted)]">
        <p>
          This instrument reports what propagated. It does not report whether you
          succeeded, and it cannot tell you which of your actions mattered — both are
          consequences of the theory it implements rather than missing features.
        </p>
      </footer>
    </main>
  )
}
