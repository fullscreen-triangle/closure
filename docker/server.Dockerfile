# syntax=docker/dockerfile:1.7
# ---------------------------------------------------------------------------
# closure-server
#
# Two stages: a builder that compiles the workspace against a cached
# dependency layer, and a distroless runtime that carries the binary and
# nothing else.
# ---------------------------------------------------------------------------

# --- build -----------------------------------------------------------------
FROM rust:1.97-slim-bookworm AS builder

WORKDIR /build

# Compile dependencies first, against stub sources, so that editing our own
# code does not invalidate the dependency layer.
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/closure-kernel/Cargo.toml   crates/closure-kernel/
COPY crates/closure-runtime/Cargo.toml  crates/closure-runtime/
COPY crates/closure-cli/Cargo.toml      crates/closure-cli/
COPY crates/closure-server/Cargo.toml   crates/closure-server/
RUN mkdir -p crates/closure-kernel/src crates/closure-runtime/src \
             crates/closure-cli/src crates/closure-server/src \
 && echo 'pub fn stub() {}' > crates/closure-kernel/src/lib.rs \
 && echo 'pub fn stub() {}' > crates/closure-runtime/src/lib.rs \
 && echo 'fn main() {}'     > crates/closure-cli/src/main.rs \
 && echo 'fn main() {}'     > crates/closure-server/src/main.rs \
 && cargo build --release -p closure-server 2>/dev/null || true

# Now the real sources.
COPY crates crates
RUN find crates -name '*.rs' -newermt '@0' -exec touch {} + \
 && cargo build --release -p closure-server \
 && strip target/release/closure-server

# --- runtime ---------------------------------------------------------------
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

LABEL org.opencontainers.image.title="closure-server" \
      org.opencontainers.image.description="Host for the closure runtime" \
      org.opencontainers.image.licenses="AGPL-3.0-or-later" \
      org.opencontainers.image.source="https://github.com/kundai/closure"

COPY --from=builder /build/target/release/closure-server /usr/local/bin/closure-server
COPY --chown=nonroot:nonroot zuerich/data/*.json /data/

USER nonroot
EXPOSE 8080

ENV CLOSURE_BIND=0.0.0.0:8080 \
    CLOSURE_DATA_DIR=/data \
    RUST_LOG=closure_server=info

ENTRYPOINT ["/usr/local/bin/closure-server"]
