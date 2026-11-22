# syntax=docker/dockerfile:1

# ---- build ----
FROM rust:1-bookworm AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release -p backers --locked

# ---- runtime ----
FROM debian:bookworm-slim
# ca-certificates: required for outbound HTTPS (webhooks, OAuth, SMTP).
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system backers \
    && useradd --system --gid backers --home-dir /data \
        --shell /usr/sbin/nologin backers \
    && mkdir -p /data \
    && chown backers:backers /data

COPY --from=builder /src/target/release/backers /usr/local/bin/backers

ENV BACKERS_DATA=/data \
    PORT=8080
VOLUME ["/data"]
EXPOSE 8080

USER backers
CMD ["backers", "serve"]
