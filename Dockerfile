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

RUN --mount=type=cache,id=yanshi-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
  --mount=type=cache,id=yanshi-cargo-git,target=/usr/local/cargo/git,sharing=locked \
  --mount=type=cache,id=yanshi-server-target,target=/app/target,sharing=locked \
  CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS}" cargo build --release -p "${PACKAGE}" --bin "${BIN}" \
  && mkdir -p /out \
  && cp "/app/target/release/${BIN}" /out/yanshi

FROM debian:bookworm-slim AS runtime

ARG INSTALL_MULTIMODAL_TOOLS=1
ARG TYPST_VERSION=0.13.1
ARG PANDOC_VERSION=3.6.4

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
    && dpkg_arch="$(dpkg --print-architecture)" \
    && case "${dpkg_arch}" in \
      amd64) typst_arch="x86_64"; pandoc_arch="amd64" ;; \
      arm64) typst_arch="aarch64"; pandoc_arch="arm64" ;; \
      *) typst_arch=""; pandoc_arch="" ;; \
    esac \
    && if [ -n "${pandoc_arch}" ]; then \
      curl -fsSL --retry 3 "https://github.com/jgm/pandoc/releases/download/${PANDOC_VERSION}/pandoc-${PANDOC_VERSION}-1-${pandoc_arch}.deb" \
        -o /tmp/pandoc.deb \
      && dpkg -i /tmp/pandoc.deb \
      && rm -f /tmp/pandoc.deb; \
    else \
      apt-get install -y --no-install-recommends pandoc; \
    fi \
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
  && mkdir -p \
    /data/runs \
    /data/traces \
    /data/workspace \
    /etc/yanshi \
    /home/yanshi/.yanshi/traces \
    /home/yanshi/.yanshi/marketplace/cache \
    /home/yanshi/.yanshi/skills \
    /home/yanshi/.yanshi/plugins \
  && chown -R yanshi:yanshi /data /home/yanshi/.yanshi

COPY config/pricing.prod.yml /etc/yanshi/pricing.yml
COPY --from=builder /out/yanshi /usr/local/bin/yanshi

USER yanshi
ENV PORT=3000
ENV RUST_LOG=info
EXPOSE 3000

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/yanshi"]
