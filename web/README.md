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
  App.tsx              join, then session
  lib/api.ts           typed client; mirrors closure-server routes
  components/
    JoinForm.tsx       paste a token
    SessionPanel.tsx   what the session has accumulated
```

Types in `lib/api.ts` are hand-mirrored from the Rust route handlers. If you
change a response shape in `closure-server`, change it here too — the CI
typecheck will not catch a drift between the two.
