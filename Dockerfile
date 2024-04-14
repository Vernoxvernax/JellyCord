FROM rust:1.72 as build

RUN mkdir -p /build/src && \
    mkdir -p /build/migrations
COPY Cargo.toml sqlx-data.json /build/
COPY src/* /build/src/
COPY migrations/* /build/migrations/
RUN CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install --path /build/.

FROM rust:1.72
COPY --from=build /usr/local/cargo/bin/jellycord /usr/local/cargo/bin/jellycord
COPY docker_build/entrypoint.sh /entrypoint.sh
RUN mkdir /data
VOLUME ["/data"]
ENTRYPOINT ["/bin/bash", "/entrypoint.sh"]
