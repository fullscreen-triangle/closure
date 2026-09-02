# syntax=docker/dockerfile:1.7
# ---------------------------------------------------------------------------
# closure web surface
#
# Vite build, served by nginx. The API is reached through a reverse proxy so
# the browser sees one origin.
# ---------------------------------------------------------------------------

# --- build -----------------------------------------------------------------
FROM node:24-alpine AS builder

WORKDIR /build

# Dependencies first, so that editing source does not refetch them.
COPY web/package.json web/package-lock.json* ./
RUN npm ci --no-audit --no-fund 2>/dev/null || npm install --no-audit --no-fund

COPY web/ ./
RUN npm run build

# --- runtime ---------------------------------------------------------------
FROM nginx:1.27-alpine AS runtime

LABEL org.opencontainers.image.title="closure-web" \
      org.opencontainers.image.description="Web interaction surface for closure" \
      org.opencontainers.image.licenses="AGPL-3.0-or-later"

COPY --from=builder /build/dist /usr/share/nginx/html
COPY docker/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 8081

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s \
  CMD wget -qO- http://127.0.0.1:8081/ >/dev/null 2>&1 || exit 1
