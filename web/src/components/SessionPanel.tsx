import type { SessionView } from '../lib/api'

interface Props {
  session: SessionView
  token: string
}

/**
 * What the session has accumulated.
 *
 * Every figure shown here is a fact about the run rather than a judgement of
 * it. The record is monotone and never falls; the fingerprint is what makes a
 * session reproducible, since the trajectory is not. There is deliberately
 * nothing here that reads as a score.
 *
 * When the runtime holds nothing, that is said in words rather than hidden.
 * A panel that quietly dropped its empty figures would let a reader assume
 * they were being computed and were merely small.
 */
export function SessionPanel({ session, token }: Props) {
  return (
    <section>
      <dl className="grid grid-cols-2 gap-x-6 gap-y-5 border-y border-[var(--rule)] py-5 text-sm sm:grid-cols-4">
        <Figure label="city" value={session.city} />
        <Figure label="tick" value={String(session.tick)} />
        <Figure label="posts" value={String(session.posts)} />
        <Figure
          label="voices"
          value={String(session.voices)}
          hint="handles, not people"
        />
        <Figure label="nodes" value={String(session.nodes)} />
        <Figure
          label="record"
          value={String(session.record)}
          hint="monotone; never falls"
        />
        <Figure
          label="protocol"
          value={session.protocol_fingerprint}
          hint="what is reproducible"
        />
        <Figure
          label="seed"
          value={String(session.seed)}
          hint="restate this to reopen the same square"
        />
      </dl>

      {session.nodes === 0 && (
        <p className="mt-3 text-xs text-[var(--muted)]">
          The runtime holds no nodes in this session yet — nothing has propagated into
          it. It fills once the world reports itself.
        </p>
      )}

      <p className="mt-3 font-mono text-xs text-[var(--muted)]">{token}</p>
    </section>
  )
}

function Figure({
  label,
  value,
  hint,
}: {
  label: string
  value: string
  hint?: string
}) {
  return (
    <div>
      <dt className="text-xs uppercase tracking-wider text-[var(--muted)]">{label}</dt>
      <dd className="mt-1 break-all font-mono">{value}</dd>
      {hint && <dd className="mt-1 text-xs text-[var(--muted)]">{hint}</dd>}
    </div>
  )
}
