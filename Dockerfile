# Stage 1: Build Rust binaries
# Pinned to 1.85 to match workspace `rust-version` in Cargo.toml.
FROM rust:1.85-slim-bookworm AS rust-builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev clang && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
# Build the distributed node binary and the legacy HTTP gateway.
RUN cargo build --release --bin exochain --bin exo-gateway

# Stage 2: Build frontend
FROM node:20-slim AS web-builder
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Copy both binaries — `exochain` is the primary entrypoint;
# `exo-gateway` is the standalone gateway for environments that prefer it.
COPY --from=rust-builder /app/target/release/exochain /app/
COPY --from=rust-builder /app/target/release/exo-gateway /app/
COPY --from=web-builder /app/web/dist /app/web/dist
COPY crates/exo-gateway/migrations /app/migrations
# Bundle the entrypoint script so env-var driven configuration works
# regardless of which start-command override is in effect.
COPY deploy/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh && ln -s /app/exochain /usr/local/bin/exochain

# Default data directory inside the container.
ENV EXOCHAIN_DATA_DIR=/data
ENV RUST_LOG=info

# P2P (TCP + QUIC) and API
EXPOSE 4001 4002 8080

# Persistent state (identity key + DAG) lives at /data.
# On Railway, /data is mounted via a Railway volume — do NOT use the
# Dockerfile VOLUME keyword (Railway bans it).
# For plain Docker: `docker run -v exochain-data:/data exochain/node`.

CMD ["/app/entrypoint.sh"]
