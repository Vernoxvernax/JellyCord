FROM lukemathwalker/cargo-chef:latest-rust-alpine AS chef
WORKDIR /build

FROM chef AS planner
COPY Cargo.toml ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apk add --no-cache libressl-dev
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo install sqlx-cli
RUN sqlx database create && sqlx migrate run
RUN cargo install --path /build/.

FROM alpine:3.21.3 AS runtime
COPY --from=builder /usr/local/cargo/bin/jellycord /usr/local/cargo/bin/jellycord
COPY docker_build/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh && mkdir /data
VOLUME ["/data"]
ENTRYPOINT ["/entrypoint.sh"]
