import type { Heartbeat } from '../lib/hooks'
import type { Observation } from '../lib/api'

/**
 * The tick, and what the world reported.
 *
 * **The auto-tick is a client policy.** Nothing on the server runs on a
 * timer; the world moves when a client asks it to, which is what makes a run
 * a reproducible sequence of requests. So the control says what it is doing
 * in words rather than showing a play/pause pair — media transport controls
 * imply a recording being played back, and there is no recording.
 *
 * The tick is shown plainly. Never `27 / 100`, never a progress bar: a bar
 * implies a terminus, and there is no state after which the square has
 * nothing left to say (Thm 7.4), nor any exit code to compute (Thm 11.5).
 */
export function WorldClock({
  tick,
  beat,
  weather,
}: {
  tick: number
  beat: Heartbeat
  weather: Observation | null
}) {
  return (
    <div className="mt-6 flex flex-wrap items-center gap-x-4 gap-y-2 border-y border-[var(--rule)] py-3 text-xs">
      <span className="font-mono">tick {tick}</span>

      <span className="text-[var(--muted)]">
        {beat.running ? 'the square is moving on its own' : 'the square is held'}
      </span>

      <button
        type="button"
        onClick={() => {
          beat.setRunning(!beat.running)
        }}
        className="border border-[var(--rule)] px-2 py-1 lowercase text-[var(--muted)] hover:text-[var(--ink)]"
      >
        {beat.running ? 'hold' : 'let it move'}
      </button>

      {!beat.running && (
        <button
          type="button"
          onClick={beat.step}
          disabled={beat.busy}
          className="border border-[var(--rule)] px-2 py-1 lowercase text-[var(--muted)] hover:text-[var(--ink)] disabled:opacity-50"
        >
          one step
        </button>
      )}

      {weather && (
        <span className="text-[var(--muted)]">
          {/* Display only. Whatever this reading did is already in the
              termini of the posts the city made. */}
          {weather.temperature_c.toFixed(1)} °C, wind {weather.wind_kph.toFixed(0)} km/h
          — {weather.source}
        </span>
      )}

      {beat.error && (
        <span className="text-[var(--muted)]" role="status">
          {beat.error}
        </span>
      )}
    </div>
  )
}
