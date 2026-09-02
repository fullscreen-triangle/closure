# The `closure` command

The CLI does three things: it authenticates you against a host, it mints a
session token for a city, and it opens the web surface. The world itself —
the agents, their graphs, their records — lives on the host, so a session
survives you closing your laptop and several people can inhabit one city.

## Install

```bash
cargo install --path crates/closure-cli
```

Or download a binary from the releases page. There is one executable,
`closure`, with no runtime dependencies.

## First run

```bash
closure login                  # authenticate this machine
closure session new            # mint a token, open the browser
```

That prints something like:

```
  Session opened in zuerich.

      CLOSURE-K4M2-9XPQ-7RTF

  Paste that at https://play.closure.city
```

## Commands

| Command | What it does |
|---|---|
| `closure login` | Authenticate this machine against the host |
| `closure logout` | Forget the stored credential |
| `closure session new` | Mint a token and open the web surface |
| `closure session show` | Show the current session token |
| `closure cities` | List the cities this host can instantiate |
| `closure doctor` | Report the six implementation invariants |
| `closure where` | Print where config and credentials are stored |

### Useful flags

| Flag | Effect |
|---|---|
| `--host <url>` | Point at a different host (`CLOSURE_HOST`) |
| `--json` | Machine-readable output, for scripts |
| `-v`, `-vv`, `-vvv` | Increase logging |
| `session new --quiet` | Print only the token |
| `session new --no-open` | Do not launch a browser |

## `closure doctor`

Worth running once. It prints the six invariants the runtime honours, what
predicate each requires, and which theorem certifies it — followed by the
three things the instrument refuses to report and why each refusal is a
consequence of the theory rather than a missing feature.

Note that `doctor` issues no pass/fail verdict on your session. It cannot:
Invariant 6 forbids the system from comparing an achieved determination to an
expected one. It reports the commitments themselves.

## Configuration

Stored under the platform config directory; `closure where` prints the exact
path. On Linux that is `~/.config/closure/config.toml`, on macOS
`~/Library/Application Support/city.closure.closure/`, on Windows
`%APPDATA%\closure\closure\config`.

```toml
credential = "..."                    # written by `closure login`
host = "https://api.closure.city"
web = "https://play.closure.city"
```

Set `CLOSURE_CONFIG_DIR` to override the location — useful for testing, and
for keeping several identities apart.

The file is written with `0600` permissions where the platform supports it.

## Running against a local host

```bash
docker compose up --build
closure --host http://localhost:8080 login
closure --host http://localhost:8080 session new
```

Or set `CLOSURE_HOST` once and drop the flag.

## Scripting

```bash
TOKEN=$(closure session new --quiet --no-open)
curl -s "$CLOSURE_HOST/v1/session/$TOKEN" | jq
```

`--json` on any command gives structured output; the shapes match the API
types in `web/src/lib/api.ts`.
