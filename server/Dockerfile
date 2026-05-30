# Builder
FROM rust:1.82-slim AS builder
WORKDIR /app

# Cache deps separately from sources for fast incremental builds
COPY Cargo.toml ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY src ./src
RUN cargo build --release --bin password-wallet-server

# Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/password-wallet-server /usr/local/bin/password-wallet-server

# Self-hosters MUST override JWT_SECRET in their compose file / k8s secret.
# Refuses to start in release without it.
ENV BIND=0.0.0.0:8090
ENV DB_PATH=/data/vault-server.db
VOLUME /data
EXPOSE 8090

ENTRYPOINT ["/usr/local/bin/password-wallet-server"]
