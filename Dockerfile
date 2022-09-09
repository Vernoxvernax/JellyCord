FROM rust:1.63

RUN mkdir -p /build/src
RUN mkdir -p /build/migrations
COPY Cargo.toml sqlx-data.json /build/
COPY src/main.rs /build/src/.
COPY migrations/* /build/migrations/
RUN cargo install --path /build/.
RUN mkdir /data
COPY docker_build/entrypoint.sh /entrypoint.sh
VOLUME ["/data"]
ENTRYPOINT ["/bin/bash", "/entrypoint.sh"]
