# syntax=docker/dockerfile:1.7
FROM rust:1.88-bookworm AS builder
ENV CARGO_BUILD_JOBS=2
WORKDIR /src
RUN apt-get update && apt-get install -y --no-install-recommends build-essential ca-certificates clang cmake libssl-dev pkg-config python3 && rm -rf /var/lib/apt/lists/*
COPY . .
RUN --mount=type=cache,id=alvenqis-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=alvenqis-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=alvenqis-cargo-target,target=/src/target,sharing=locked \
    cargo build --locked --release \
      -p alvenqis-node \
      -p alvenqis-rpc-gateway \
      -p alvenqis-indexer \
      -p alvenqis-vps-admin \
      -p alvenqis-mining-pool \
    && install -Dm755 target/release/alvenqis-node /out/alvenqis-node \
    && install -Dm755 target/release/alvenqis-rpc-gateway /out/alvenqis-rpc-gateway \
    && install -Dm755 target/release/alvenqis-indexer /out/alvenqis-indexer \
    && install -Dm755 target/release/alvenqis-vps-admin /out/alvenqis-vps-admin \
    && install -Dm755 target/release/alvenqis-mining-pool /out/alvenqis-mining-pool
FROM debian:bookworm-slim AS runtime
ARG ALVENQIS_UID=10001
ARG ALVENQIS_GID=10001
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl gettext-base libssl3 procps tini && rm -rf /var/lib/apt/lists/* && groupadd --gid "${ALVENQIS_GID}" alvenqis && useradd --uid "${ALVENQIS_UID}" --gid "${ALVENQIS_GID}" --home-dir /home/alvenqis --create-home --shell /usr/sbin/nologin alvenqis
WORKDIR /app
COPY --from=builder /out/alvenqis-node /usr/local/bin/alvenqis-node
COPY --from=builder /out/alvenqis-rpc-gateway /usr/local/bin/alvenqis-rpc-gateway
COPY --from=builder /out/alvenqis-indexer /usr/local/bin/alvenqis-indexer
COPY --from=builder /out/alvenqis-vps-admin /usr/local/bin/alvenqis-vps-admin
COPY --from=builder /out/alvenqis-mining-pool /usr/local/bin/alvenqis-mining-pool
COPY alvenqis-release/vps-control-plane/docker/entrypoint.sh /usr/local/bin/alvenqis-entrypoint
COPY alvenqis-release/vps-control-plane/docker/indexer-loop.sh /usr/local/bin/alvenqis-indexer-loop
COPY alvenqis-release/vps-control-plane/docker/pool-supervisor.sh /usr/local/bin/alvenqis-pool-supervisor
COPY alvenqis-release/vps-control-plane/docker/templates /app/templates
COPY configs /app/configs
COPY docs/release /app/docs/release
RUN chmod 0755 /usr/local/bin/alvenqis-* && mkdir -p /config /data/.alvenqis-mainnet/chain /data/.alvenqis-mainnet/mempool /data/.alvenqis-mainnet/indexer /data/.alvenqis-mainnet/node /data/control /data/pool && chown -R alvenqis:alvenqis /config /data /app
USER alvenqis
ENTRYPOINT ["/usr/bin/tini", "-g", "--", "/usr/local/bin/alvenqis-entrypoint"]
