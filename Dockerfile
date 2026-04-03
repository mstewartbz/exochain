# Stage 1: Build Rust binaries
FROM rust:1.86-slim-bookworm AS rust-builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
# Build the distributed node binary (primary) and legacy gateway.
RUN cargo build --release --bin exochain --bin decision-forum-server

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

# Copy both binaries — exochain is the primary entrypoint.
COPY --from=rust-builder /app/target/release/exochain /app/
COPY --from=rust-builder /app/target/release/decision-forum-server /app/
COPY --from=web-builder /app/web/dist /app/web/dist
COPY crates/exo-gateway/migrations /app/migrations

# Default data directory inside the container.
ENV EXOCHAIN_DATA_DIR=/data
ENV RUST_LOG=info

# P2P (TCP + QUIC) and API
EXPOSE 4001 4002 8080

# Mount a volume for persistent state (identity key + DAG).
VOLUME ["/data"]

CMD ["./exochain", "start"]
