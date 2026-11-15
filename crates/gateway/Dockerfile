# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates cmake g++ libssl-dev pkg-config \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY Cargo.toml Cargo.lock* ./
COPY src ./src

ARG CARGO_BUILD_JOBS=2
RUN --mount=type=cache,id=yanshi-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
  --mount=type=cache,id=yanshi-gateway-target,target=/src/target,sharing=locked \
  CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS}" cargo build --release \
  && mkdir -p /out \
  && cp /src/target/release/yanshi-gateway /out/yanshi-gateway

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates libssl3 tini \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /out/yanshi-gateway /usr/local/bin/yanshi-gateway

ENV RUST_LOG=info
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/usr/local/bin/yanshi-gateway"]
