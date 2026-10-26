# ── Build stage ───────────────────────────────────────────────────────────
FROM rust:1.80-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

RUN cargo build --release --workspace --exclude iad-grpc --exclude iad-wasm

# ── Runtime stage ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates curl && \
    rm -rf /var/lib/apt/lists/* && \
    useradd -r -s /bin/false -d /home/iad iad && \
    mkdir -p /home/iad/.iad && chown -R iad:iad /home/iad

COPY --from=builder /build/target/release/iad /usr/local/bin/iad

USER iad
WORKDIR /home/iad

RUN iad init

EXPOSE 7700

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s \
    CMD curl -f http://127.0.0.1:7700/healthz || exit 1

ENTRYPOINT ["iad"]
CMD ["start", "--foreground"]
