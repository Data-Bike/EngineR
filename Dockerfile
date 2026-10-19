# EngineR API + Worker multi-stage Dockerfile.
#
# Stage 1: Build all Rust workspace binaries.
# Stage 2: Runtime with minimal image.

FROM rust:1.95-bookworm AS builder

WORKDIR /build

# Install sqlx-cli for potential migration support + pkg-config + libssl
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifest + all Cargo.toml files for dependency caching.
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY apps/ ./apps/

# Build the API, worker and MCP binaries.
RUN cargo build --release --bin enginer-api --bin enginer-worker --bin enginer-mcp --bin enginer-dump

# --- Runtime stage ---
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 tini postgresql-client \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries
COPY --from=builder /build/target/release/enginer-api /app/enginer-api
COPY --from=builder /build/target/release/enginer-worker /app/enginer-worker
COPY --from=builder /build/target/release/enginer-mcp /app/enginer-mcp
COPY --from=builder /build/target/release/enginer-dump /app/enginer-dump

# Copy migrations + config
COPY migrations/ /app/migrations/
COPY config/ /app/config/
COPY Rocket.toml /app/Rocket.toml
COPY docker/entrypoint-api.sh /app/entrypoint-api.sh
COPY docker/entrypoint-worker.sh /app/entrypoint-worker.sh
RUN chmod +x /app/entrypoint-api.sh /app/entrypoint-worker.sh

ENV ENGINER_CONFIG=/app/config/enginer.docker.toml
ENV ENGINER_DATABASE_URL=postgres://enginer_app:enginer_app@postgres:5432/enginer

EXPOSE 8000

ENTRYPOINT ["tini", "--"]
CMD ["/app/entrypoint-api.sh"]
