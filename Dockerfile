FROM rust:1.80-slim AS builder

WORKDIR /usr/src/iad

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY crates/iad/Cargo.toml crates/iad/Cargo.toml
COPY crates/iad-core/Cargo.toml crates/iad-core/Cargo.toml
COPY crates/iad-storage/Cargo.toml crates/iad-storage/Cargo.toml
COPY crates/iad-api/Cargo.toml crates/iad-api/Cargo.toml
COPY crates/iad-sdk/Cargo.toml crates/iad-sdk/Cargo.toml

# Create dummy source files for dependency caching
RUN mkdir -p crates/iad/src crates/iad-core/src crates/iad-storage/src crates/iad-api/src crates/iad-sdk/src && \
    echo 'fn main() {}' > crates/iad/src/main.rs && \
    echo 'pub fn lib() {}' > crates/iad-core/src/lib.rs && \
    echo 'pub fn lib() {}' > crates/iad-storage/src/lib.rs && \
    echo 'pub fn lib() {}' > crates/iad-api/src/lib.rs && \
    echo 'pub fn lib() {}' > crates/iad-sdk/src/lib.rs && \
    cargo build --release 2>/dev/null || true

# Copy real source
COPY crates/ crates/

# Build
RUN cargo build --release

# Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/iad/target/release/iad /usr/local/bin/iad

RUN useradd --create-home --shell /bin/bash iad
USER iad
WORKDIR /home/iad

EXPOSE 7700 7701 7702

ENTRYPOINT ["iad"]
CMD ["start", "--foreground"]
