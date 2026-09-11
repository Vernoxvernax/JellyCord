FROM rust:1-slim-trixie AS builder
WORKDIR /build
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        libssl-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
COPY src ./src
COPY migrations ./migrations
COPY .env ./

RUN cargo install sqlx-cli
RUN sqlx database create
RUN sqlx migrate run
RUN cargo install --path .


FROM debian:trixie-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/jellycord /usr/local/bin/jellycord
COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx
COPY migrations /migrations
COPY docker_build/entrypoint.sh /entrypoint.sh

RUN chmod +x /entrypoint.sh \
    && mkdir /data

VOLUME ["/data"]

ENTRYPOINT ["/entrypoint.sh"]
