# EngineR Nuxt 4 admin UI Dockerfile (docs/16).
#
# Build context is the project root (so we can copy web/ API client source).

FROM node:22-bookworm-slim AS builder

WORKDIR /build

# Copy web (API client) + nuxt source
COPY web/package.json web/package-lock.json ./web/
COPY nuxt/package.json nuxt/package-lock.json ./nuxt/

# Install nuxt dependencies (includes alias to ../web/src)
WORKDIR /build/nuxt
RUN npm ci --prefer-offline 2>/dev/null || npm install

# Copy full source
COPY web/ ../web/
COPY nuxt/ ./

# Build the Nuxt app
RUN npm run build

# --- Runtime stage ---
FROM node:22-bookworm-slim AS runtime

WORKDIR /app

COPY --from=builder /build/nuxt/.output ./.output

ENV NODE_ENV=production
ENV HOST=0.0.0.0
ENV PORT=3000

EXPOSE 3000

CMD ["node", ".output/server/index.mjs"]
