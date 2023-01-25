FROM rust:1.63 as build

RUN mkdir -p /build/src && \
    mkdir -p /build/migrations
COPY Cargo.toml sqlx-data.json /build/
COPY src/* /build/src/
COPY migrations/* /build/migrations/
RUN cargo install --path /build/.

FROM rust:1.63
COPY --from=build /usr/local/cargo/bin/jellycord /usr/local/cargo/bin/jellycord
COPY docker_build/entrypoint.sh /entrypoint.sh
RUN mkdir /data
VOLUME ["/data"]
ENTRYPOINT ["/bin/bash", "/entrypoint.sh"]
