# Security

## Reporting

Report vulnerabilities privately to **kundai.sachikonye@tum.de** rather than
through a public issue. Please include what you did, what happened, and how
severe you judge it. Expect an acknowledgement within a week.

## Scope

In scope: the CLI, the server, the web surface, the token flow, and the
container images.

Out of scope: findings that require an attacker to already control the
player's machine or the host, and the deliberately permissive dev-mode CORS
configuration in `.env.example`.

## What a session token is, and is not

A token names a session on a host. It is short by design — a player has to
retype it from a terminal — and its entropy is about 77 bits over an
unambiguous alphabet.

It is **not** a credential for anything other than that session, and it is
**not** a substitute for authentication. `closure login` establishes who you
are; the token establishes which world you are joining. Do not extend a token
to carry authorisation.

Tokens appear in URLs when the CLI opens the browser for you, so treat them as
you would a share link: they will end up in browser history and possibly in a
referrer header.

## Deployment notes

The compose stack runs the server read-only, with `no-new-privileges`, as a
non-root user, from a distroless image. Keep it that way.

`CLOSURE_ALLOWED_ORIGINS` defaults to localhost. Set it explicitly in
production; do not widen it to `*`.
