FROM rust:1.90-bookworm AS builder

RUN rustup install nightly-2025-08-01
RUN rustup default nightly-2025-08-01
RUN rustup component add rustfmt clippy
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/conduit conduit
COPY --from=builder /app/public public
CMD ["./conduit"]
