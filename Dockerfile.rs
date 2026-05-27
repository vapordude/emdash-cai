# syntax=docker/dockerfile:1

# ---- Chef (dependency layer caching) ----
FROM rust:1-alpine AS chef
RUN apk add --no-cache musl-dev && cargo install cargo-chef
WORKDIR /app

# ---- Plan ----
FROM chef AS planner
COPY emdash-rs/ emdash-rs/
WORKDIR /app/emdash-rs
RUN cargo chef prepare --recipe-path recipe.json

# ---- Build dependencies ----
FROM chef AS builder-deps
WORKDIR /app/emdash-rs
COPY --from=planner /app/emdash-rs/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# ---- Build binary ----
FROM builder-deps AS builder
COPY emdash-rs/ .
RUN cargo build --release --bin emdash

# ---- Runtime ----
FROM alpine:3.21
RUN apk add --no-cache ca-certificates libgcc

WORKDIR /app
COPY --from=builder /app/emdash-rs/target/release/emdash /usr/local/bin/emdash

RUN mkdir -p /data /storage
VOLUME ["/data", "/storage"]

ENV DATABASE_URL=/data/emdash.db
ENV STORAGE_PATH=/storage
ENV PORT=3000
ENV RUST_LOG=info

EXPOSE 3000
CMD ["emdash", "serve"]
