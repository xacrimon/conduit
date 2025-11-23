FROM rust:1.90-bookworm AS builder

ARG TARGETPLATFORM
ARG TAILWIND_DOWNLOAD_URL="https://github.com/tailwindlabs/tailwindcss/releases/download/v4.1.17/tailwindcss-linux-"

RUN rustup install nightly-2025-08-01
RUN rustup default nightly-2025-08-01
RUN rustup component add rustfmt clippy

RUN if [ "$TARGETPLATFORM" = "linux/amd64" ]; then ARCHITECTURE=x64; elif [ "$TARGETPLATFORM" = "linux/arm64" ]; then ARCHITECTURE=arm64; else ARCHITECTURE=x64; fi \
    && wget -O /usr/local/bin/tailwindcss "${TAILWIND_DOWNLOAD_URL}${ARCHITECTURE}" \
    && chmod +x /usr/local/bin/tailwindcss

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
