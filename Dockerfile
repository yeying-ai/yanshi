# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder

WORKDIR /app

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates libssl-dev pkg-config protobuf-compiler \
  && rm -rf /var/lib/apt/lists/*

COPY . .

ARG PACKAGE=yanshi-server
ARG BIN=yanshi-server
ARG CARGO_BUILD_JOBS=2

RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/usr/local/cargo/git \
  --mount=type=cache,target=/app/target \
  CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS}" cargo build --release -p "${PACKAGE}" --bin "${BIN}" \
  && mkdir -p /out \
  && cp "/app/target/release/${BIN}" /out/yanshi

FROM debian:bookworm-slim AS runtime

ARG INSTALL_MULTIMODAL_TOOLS=1
ARG TYPST_VERSION=0.13.1

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libssl3 \
    tini \
    xz-utils \
  && if [ "${INSTALL_MULTIMODAL_TOOLS}" = "1" ]; then \
    apt-get install -y --no-install-recommends \
      ffmpeg \
      fonts-noto-cjk \
      pandoc \
    && dpkg_arch="$(dpkg --print-architecture)" \
    && case "${dpkg_arch}" in \
      amd64) typst_arch="x86_64" ;; \
      arm64) typst_arch="aarch64" ;; \
      *) typst_arch="" ;; \
    esac \
    && if [ -n "${typst_arch}" ]; then \
      curl -fsSL --retry 3 "https://github.com/typst/typst/releases/download/v${TYPST_VERSION}/typst-${typst_arch}-unknown-linux-musl.tar.xz" \
        -o /tmp/typst.tar.xz \
      && tar -xJf /tmp/typst.tar.xz -C /tmp \
      && install -m 0755 "/tmp/typst-${typst_arch}-unknown-linux-musl/typst" /usr/local/bin/typst \
      && rm -rf /tmp/typst*; \
    fi; \
  fi \
  && apt-get purge -y --auto-remove xz-utils \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --create-home --uid 10001 --shell /usr/sbin/nologin yanshi \
  && mkdir -p /data/runs \
  && chown -R yanshi:yanshi /data

COPY --from=builder /out/yanshi /usr/local/bin/yanshi

USER yanshi
ENV PORT=3000
ENV RUST_LOG=info
EXPOSE 3000

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/yanshi"]
