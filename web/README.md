# closure — web surface

The interaction surface. A player arrives with a token minted by the CLI,
pastes it, and is joined to a running city.

## Develop

```bash
npm install
npm run dev        # http://localhost:5173
```

The dev server proxies `/v1` to `http://localhost:8080`, so run the API
alongside it (`make dev-server`, or `make dev` for both). Override the target
with `CLOSURE_API`.

## Scripts

| Script | What it does |
|---|---|
| `npm run dev` | Dev server with HMR |
| `npm run build` | Typecheck, then production build to `dist/` |
| `npm run lint` | ESLint, zero warnings tolerated |
| `npm run fmt` | Prettier, writing in place |
| `npm run typecheck` | `tsc --noEmit` |
| `npm test` | Vitest, `node` environment, pure functions only |

## What this client does not render

There is no progress bar, no score, and no indicator of how a session is
going — not as a stylistic choice, but because the server exposes no such
value. The runtime cannot compute a verdict, and it cannot attribute a change
in the world to something a player did. A component displaying either would be
inventing a number.

What `SessionPanel` shows instead are facts about the run: the node count, the
monotone record, and the protocol fingerprint that makes a session
reproducible. See the table in the root README, or run `closure doctor`.

If you are adding a view, the useful question is *what propagated*, never
*how is the player doing*.

## Structure

```
src/
  App.tsx              join, then focus: square | thread | voice
  lib/
    api.ts             typed client; mirrors closure-server routes
    speaker.ts         reads the Speaker union; every component goes through it
    act.ts             renders an act's two directions, and its absence
    affordance.ts      what a region is like, as a word and never a colour
    hooks.ts           useAsync, usePolling, useHeartbeat
  components/
    JoinForm.tsx       paste a token, and name the city yourself
    SessionPanel.tsx   what the session has accumulated
    WorldClock.tsx     the tick, and the reading the city last reported
    Feed.tsx           roots only, with reply counts and region chips
    ThreadView.tsx     one root and its replies
    Composer.tsx       post at a terminus you pick in two steps
    VoicePanel.tsx     character -> prune -> ask
    PositionStrip.tsx  a span of the city, one cell per position
    ActMark.tsx        report / question / neither / not measured
```

There is deliberately **no router**. The app has one address — the token and
the city, already in `?token=&city=` — and a `/voice/7` route would make
voices linkable and therefore enumerable, rebuilding at the URL layer the
retrieval the API refuses. Switching views here is *focus*, not navigation,
so the square keeps moving beside an open voice panel.

The city is a **text field, not a picker**. The host draws a society from the
seed and the name and holds no list to check one against, so a dropdown would
claim bound substrates that do not exist and would make every unlisted name
look disallowed. Both halves travel in the URL because both draw the square:
a link carrying only the token lands on the form with the name still to fill
in.

The theme is **dark unconditionally**, not `prefers-color-scheme`. A reader's
operating system should not decide which of two designs they get when only
one of them is being tuned; `lib/theme.test.ts` asserts the palette sits on
bare `:root` so a media query cannot creep back in.

Types in `lib/api.ts` are hand-mirrored from the Rust route handlers. If you
change a response shape in `closure-server`, change it here too — the CI
typecheck will not catch a drift between the two.
