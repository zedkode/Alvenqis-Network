# syntax=docker/dockerfile:1.7
FROM rust:1.88-bookworm AS builder

ENV CARGO_BUILD_JOBS=2
WORKDIR /src

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
      build-essential \
      ca-certificates \
      cmake \
      git \
      perl \
      pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Blockchain-prototype .

RUN --mount=type=cache,id=alvenqis-gateway-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=alvenqis-gateway-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=alvenqis-gateway-cargo-target,target=/src/target,sharing=locked \
    cargo build --locked --release -p alvenqis-pingora-gateway \
    && install -Dm755 target/release/alvenqis-pingora-gateway /out/alvenqis-pingora-gateway

FROM debian:bookworm-slim AS runtime

ARG ALVENQIS_UID=10001
ARG ALVENQIS_GID=10001

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl tini \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid "${ALVENQIS_GID}" alvenqis \
    && useradd --uid "${ALVENQIS_UID}" --gid "${ALVENQIS_GID}" \
      --home-dir /home/alvenqis --create-home --shell /usr/sbin/nologin alvenqis

COPY --from=builder /out/alvenqis-pingora-gateway /usr/local/bin/alvenqis-pingora-gateway

USER alvenqis
EXPOSE 8080 9091 10443
ENTRYPOINT ["/usr/bin/tini", "-g", "--", "/usr/local/bin/alvenqis-pingora-gateway"]
