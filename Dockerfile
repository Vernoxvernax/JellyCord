FROM lukemathwalker/cargo-chef:0.1.67-rust-slim AS chef
WORKDIR /build

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt update -y && apt install libssl-dev pkg-config -y
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo install sqlx-cli && sqlx database create && sqlx migrate run
RUN CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install --path /build/.

FROM debian:bookworm-slim AS runtime
COPY --from=builder /usr/local/cargo/bin/jellycord /usr/local/cargo/bin/jellycord
COPY docker_build/entrypoint.sh /entrypoint.sh
RUN apt update -y && \
    apt install libssl-dev curl -y && \
    apt clean -y && \
    rm -rf /var/lib/apt/lists/*
RUN mkdir /data
VOLUME ["/data"]
ENTRYPOINT ["/bin/bash", "/entrypoint.sh"]
