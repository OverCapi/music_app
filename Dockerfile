FROM rust:1-slim AS build
WORKDIR /src
# The workspace lists app/src-tauri, so its manifest and sources must exist even though it is not built.
COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY crates crates
COPY app/src-tauri app/src-tauri
RUN cargo build --release -p music-group-server --locked

FROM debian:stable-slim
RUN useradd --system --create-home --home-dir /data app \
    && apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/music-group-server /usr/local/bin/
ENV MUSIC_SERVER_ADDR=0.0.0.0:8787 MUSIC_SERVER_DB=/data/server.db
VOLUME /data
USER app
EXPOSE 8787
CMD ["music-group-server"]
