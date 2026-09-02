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
 */
export function SessionPanel({ session, token }: Props) {
  return (
    <section className="space-y-8">
      <div>
        <p className="text-sm opacity-60">You are in</p>
        <p className="text-3xl">{session.city}</p>
      </div>

      <dl className="grid grid-cols-2 gap-6 border-y border-[var(--rule)] py-6 text-sm sm:grid-cols-4">
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
        <Figure label="opened" value={new Date(session.opened).toLocaleString()} />
      </dl>

      <p className="font-mono text-xs opacity-40">{token}</p>

      <p className="text-sm leading-relaxed opacity-70">
        The conversation surface attaches here. Agents divide their attention across
        scenes you cannot see, so one may answer you thinly, or not at that moment at
        all. That is the scheduler, not a fault.
      </p>
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
      <dt className="text-xs uppercase tracking-wider opacity-50">{label}</dt>
      <dd className="mt-1 break-all font-mono">{value}</dd>
      {hint && <dd className="mt-1 text-xs opacity-40">{hint}</dd>}
    </div>
  )
}
