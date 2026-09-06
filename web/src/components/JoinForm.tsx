import { useState, type FormEvent } from 'react'

interface Props {
  onJoin: (token: string) => void | Promise<void>
  busy: boolean
  error: string | null
}

/** Paste the token the CLI printed. */
export function JoinForm({ onJoin, busy, error }: Props) {
  const [value, setValue] = useState('')

  function submit(e: FormEvent) {
    e.preventDefault()
    if (value.trim()) void onJoin(value)
  }

  return (
    <form onSubmit={submit} className="space-y-4">
      <label htmlFor="token" className="block text-sm text-[var(--muted)]">
        Paste the token from <code className="font-mono">closure session new</code>
      </label>

      <input
        id="token"
        name="token"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder="CLOSURE-XXXX-XXXX-XXXX"
        autoComplete="off"
        spellCheck={false}
        disabled={busy}
        className="w-full rounded border border-[var(--rule)] bg-transparent px-4 py-3 font-mono text-lg tracking-wider outline-none focus:border-current disabled:opacity-50"
      />

      <button
        type="submit"
        disabled={busy || !value.trim()}
        className="rounded border border-current px-5 py-2 text-sm disabled:opacity-40"
      >
        {busy ? 'joining' : 'enter the city'}
      </button>

      {error && (
        <p role="alert" className="text-sm opacity-80">
          {error}
        </p>
      )}

      <p className="pt-6 text-sm leading-relaxed text-[var(--muted)]">
        No token? Install the CLI, then run{' '}
        <code className="font-mono">closure login</code> followed by{' '}
        <code className="font-mono">closure session new</code>.
      </p>
    </form>
  )
}
