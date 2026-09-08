import { useState, type FormEvent } from 'react'

interface Props {
  onJoin: (token: string, city: string) => void | Promise<void>
  /** A token already in hand, from the URL the CLI opened. */
  initialToken?: string
  busy: boolean
  error: string | null
}

/**
 * Paste the token the CLI printed, and say where you are.
 *
 * The city is a free text field and not a picker, because the host has no
 * list to pick from. A society is drawn from the seed and the name, so every
 * name opens a world and none opens a prepared one — a dropdown of ten
 * cities would claim ten bound substrates that do not exist, and would make
 * every other name look disallowed.
 *
 * The name is not decoration: it seeds the square along with the token, so
 * writing a different one opens a different society, and restating the pair
 * reopens the same one.
 */
export function JoinForm({ onJoin, initialToken = '', busy, error }: Props) {
  const [value, setValue] = useState(initialToken)
  const [city, setCity] = useState('')

  const name = city.trim()
  const ready = value.trim().length > 0 && name.length > 0

  function submit(e: FormEvent) {
    e.preventDefault()
    if (ready) void onJoin(value, name)
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

      <label htmlFor="city" className="block pt-2 text-sm text-[var(--muted)]">
        Name the city
      </label>

      <input
        id="city"
        name="city"
        value={city}
        onChange={(e) => setCity(e.target.value)}
        placeholder="anywhere at all"
        autoComplete="off"
        maxLength={64}
        disabled={busy}
        className="w-full rounded border border-[var(--rule)] bg-transparent px-4 py-3 text-lg outline-none focus:border-current disabled:opacity-50"
      />

      <p className="text-sm leading-relaxed text-[var(--muted)]">
        Any name. Nowhere is modelled and no name is a place someone prepared —
        the square is drawn from your token and what you call it. Write the same
        pair again to walk back into the same city.
      </p>

      <button
        type="submit"
        disabled={busy || !ready}
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
