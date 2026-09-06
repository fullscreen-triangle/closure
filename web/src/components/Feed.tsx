import { useEffect, useMemo, useRef, useState } from 'react'
import type { PostView, SubgroupView } from '../lib/api'
import { speakerKind, speakerLabel, speakerVoice } from '../lib/speaker'
import { ActMark } from './ActMark'

/**
 * The square.
 *
 * **Roots only**, with a reply count. Each tick adds a three-post thread; a
 * flat list renders that as three unrelated entries by three strangers, which
 * is exactly the wrong reading — they are one character talking to itself.
 *
 * **No order control.** The server has no viewer at this endpoint and so
 * serves `recent` for both orderings, whatever it is asked. A "nearest"
 * toggle would be a control that changes nothing.
 *
 * **No ranking, no counting.** Reply counts are counts of replies, not
 * scores; nothing here is sorted by act, and nothing aggregates across posts
 * (Prop. 9.6).
 */

function kindLabel(kind: ReturnType<typeof speakerKind>): string {
  switch (kind) {
    case 'player':
      return 'you'
    case 'voice':
      return 'a voice'
    case 'agent':
      return 'a character, to itself'
    case 'world':
      return 'the city reporting itself'
  }
}

export function Feed({
  posts,
  subgroups,
  region,
  onRegion,
  onThread,
  onVoice,
  stale,
}: {
  posts: PostView[]
  subgroups: SubgroupView[]
  region: string | null
  onRegion: (r: string | null) => void
  onThread: (root: number) => void
  onVoice: (id: number, from: number) => void
  stale: string | null
}) {
  const replies = useMemo(() => {
    const n = new Map<number, number>()
    for (const p of posts) {
      if (p.parent !== null) n.set(p.parent, (n.get(p.parent) ?? 0) + 1)
    }
    return n
  }, [posts])

  const span = useMemo(() => {
    const g = subgroups.find((s) => s.name === region)
    return g ? new Set(g.positions) : null
  }, [subgroups, region])

  const roots = useMemo(
    () => posts.filter((p) => p.parent === null && (!span || span.has(p.terminus))),
    [posts, span],
  )

  // New posts arrive at the top. If the reader has scrolled away, say so
  // instead of splicing text in above where they are reading.
  const [atTop, setAtTop] = useState(true)
  const [seen, setSeen] = useState(roots.length)
  const first = useRef(roots[0]?.id ?? null)
  useEffect(() => {
    const on = () => {
      setAtTop(window.scrollY < 40)
    }
    on()
    window.addEventListener('scroll', on, { passive: true })
    return () => {
      window.removeEventListener('scroll', on)
    }
  }, [])
  useEffect(() => {
    if (atTop) {
      setSeen(roots.length)
      first.current = roots[0]?.id ?? null
    }
  }, [atTop, roots])
  const unseen = atTop ? 0 : Math.max(0, roots.length - seen)

  return (
    <section className="mt-10">
      <div className="flex flex-wrap items-baseline gap-x-4 gap-y-2 border-b border-[var(--rule)] pb-3">
        <h2 className="font-mono text-xs uppercase tracking-[0.2em] text-[var(--muted)]">
          the square
        </h2>
        <p className="text-xs text-[var(--muted)]">
          {region
            ? 'you are listening in one region'
            : 'you are listening to the whole city'}
        </p>
      </div>

      <div className="mt-4 flex flex-wrap gap-2">
        <RegionChip
          label="everywhere"
          on={region === null}
          onClick={() => {
            onRegion(null)
          }}
        />
        {/* Never sorted by voice count — this is the city's geography, and a
            region is where you listen, not a category you rank. */}
        {subgroups.map((g) => (
          <RegionChip
            key={g.name}
            label={g.name}
            on={region === g.name}
            onClick={() => {
              onRegion(g.name)
            }}
          />
        ))}
      </div>

      {unseen > 0 && (
        <button
          type="button"
          className="mt-4 w-full border border-[var(--rule)] px-3 py-2 text-xs text-[var(--muted)] hover:text-[var(--ink)]"
          onClick={() => {
            window.scrollTo({ top: 0 })
          }}
        >
          {unseen} new · show
        </button>
      )}

      {stale && (
        <p className="mt-4 text-xs text-[var(--muted)]" role="status">
          {stale} — showing the last thing the square said.
        </p>
      )}

      <ol
        className="mt-2"
        aria-live={atTop ? 'polite' : 'off'}
        aria-label="posts in the square"
      >
        {roots.map((p) => (
          <Entry
            key={p.id}
            post={p}
            replies={replies.get(p.id) ?? 0}
            onThread={onThread}
            onVoice={onVoice}
          />
        ))}
      </ol>

      {roots.length === 0 && (
        <p className="mt-6 text-sm text-[var(--muted)]">
          Nothing has registered where you are listening. That is a fact about this
          region, not about the city.
        </p>
      )}
    </section>
  )
}

function RegionChip({
  label,
  on,
  onClick,
}: {
  label: string
  on: boolean
  onClick: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={on}
      className={
        on
          ? 'border border-[var(--ink)] px-2 py-1 font-mono text-xs lowercase'
          : 'border border-[var(--rule)] px-2 py-1 font-mono text-xs lowercase text-[var(--muted)] hover:text-[var(--ink)]'
      }
    >
      {label}
    </button>
  )
}

function Entry({
  post,
  replies,
  onThread,
  onVoice,
}: {
  post: PostView
  replies: number
  onThread: (root: number) => void
  onVoice: (id: number, from: number) => void
}) {
  const kind = speakerKind(post.speaker)
  const voice = speakerVoice(post.speaker)
  return (
    <li className="border-b border-[var(--rule)] py-4">
      <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1 text-xs text-[var(--muted)]">
        <span className="font-mono">{speakerLabel(post.speaker)}</span>
        <span>{kindLabel(kind)}</span>
        <span className="font-mono">at {post.terminus}</span>
        <ActMark act={post.act} />
      </div>

      <p className="mt-2 leading-relaxed">{post.body}</p>

      <div className="mt-2 flex flex-wrap gap-x-4 text-xs">
        {replies > 0 && (
          <button
            type="button"
            className="text-[var(--muted)] underline-offset-4 hover:text-[var(--ink)] hover:underline"
            onClick={() => {
              onThread(post.id)
            }}
          >
            {replies} {replies === 1 ? 'reply' : 'replies'}
          </button>
        )}
        {/* Entry to a character is only ever from a post you are reading.
            There is no "browse all voices": that is a catalogue. */}
        {voice !== null && (
          <button
            type="button"
            className="text-[var(--muted)] underline-offset-4 hover:text-[var(--ink)] hover:underline"
            onClick={() => {
              onVoice(voice, post.id)
            }}
          >
            who is behind this voice?
          </button>
        )}
        {kind === 'world' && (
          <span className="text-[var(--muted)]">
            the city does not reply, and nothing here is aimed at you
          </span>
        )}
      </div>
    </li>
  )
}
