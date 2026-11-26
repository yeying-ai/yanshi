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

# Build the Dioxus 0.7 WASM Web UI once natively on the host build platform
# (WASM output is architecture-independent, so multi-arch buildx reuses it).
FROM --platform=$BUILDPLATFORM rust:1-bookworm AS ui-builder

WORKDIR /app

ARG BUILD_DX_UI=1
ARG DX_CLI_VERSION=0.7.10
ARG TAILWIND_VERSION=v4.1.18

COPY yanshi-ui-dx ./yanshi-ui-dx
COPY examples/generators ./examples/generators

RUN --mount=type=cache,id=yanshi-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
  --mount=type=cache,id=yanshi-cargo-git,target=/usr/local/cargo/git,sharing=locked \
  --mount=type=cache,id=yanshi-ui-dx-target,target=/app/yanshi-ui-dx/target,sharing=locked \
  mkdir -p /ui-out \
  && if [ "${BUILD_DX_UI}" = "1" ]; then \
    apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl python3 \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add wasm32-unknown-unknown \
    && arch="$(uname -m)" \
    && case "${arch}" in \
      x86_64) dx_triple="x86_64-unknown-linux-gnu" ;; \
      aarch64|arm64) dx_triple="aarch64-unknown-linux-gnu" ;; \
      *) echo "Unsupported build arch for dx CLI: ${arch}" >&2; exit 1 ;; \
    esac \
    && curl -fsSL --retry 3 "https://github.com/DioxusLabs/dioxus/releases/download/v${DX_CLI_VERSION}/dx-${dx_triple}.tar.gz" \
      -o /tmp/dx.tar.gz \
    && mkdir -p /tmp/dx-extract \
    && tar -xzf /tmp/dx.tar.gz -C /tmp/dx-extract \
    && dx_bin="$(find /tmp/dx-extract -type f -name dx | head -n 1)" \
    && install -m 0755 "${dx_bin}" /usr/local/bin/dx \
    && rm -rf /tmp/dx.tar.gz /tmp/dx-extract \
    && TAILWIND_VERSION="${TAILWIND_VERSION}" DX_BASE_PATH="ui" ./yanshi-ui-dx/scripts/build.sh \
    && cp -R /app/yanshi-ui-dx/dist/public/. /ui-out/; \
  fi

FROM debian:bookworm-slim AS runtime

ARG INSTALL_MULTIMODAL_TOOLS=1
ARG TYPST_VERSION=0.13.1
ARG PANDOC_VERSION=3.6.4
ARG D2_VERSION=0.9.0

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
      poppler-utils \
      python3 \
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
      && rm -f /tmp/pandoc.deb \
      && curl -fsSL --retry 3 "https://github.com/terrastruct/d2/releases/download/v${D2_VERSION}/d2-v${D2_VERSION}-linux-${pandoc_arch}.tar.gz" \
        -o /tmp/d2.tar.gz \
      && mkdir -p /tmp/d2-extract \
      && tar -xzf /tmp/d2.tar.gz -C /tmp/d2-extract \
      && d2_bin="$(find /tmp/d2-extract -type f -name d2 | head -n 1)" \
      && install -m 0755 "${d2_bin}" /usr/local/bin/d2 \
      && rm -rf /tmp/d2.tar.gz /tmp/d2-extract; \
    else \
      apt-get install -y --no-install-recommends pandoc; \
    fi \
    && if [ -n "${typst_arch}" ]; then \
      curl -fsSL --retry 3 "https://github.com/typst/typst/releases/download/v${TYPST_VERSION}/typst-${typst_arch}-unknown-linux-musl.tar.xz" \
        -o /tmp/typst.tar.xz \
      && tar -xJf /tmp/typst.tar.xz -C /tmp \
      && install -m 0755 "/tmp/typst-${typst_arch}-unknown-linux-musl/typst" /usr/local/bin/typst \
      && rm -rf /tmp/typst*; \
    fi \
    && curl -fsSL --retry 3 "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp" \
      -o /tmp/yt-dlp \
    && install -m 0755 /tmp/yt-dlp /usr/local/bin/yt-dlp \
    && rm -f /tmp/yt-dlp; \
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
COPY --from=ui-builder /ui-out /usr/share/yanshi/ui

USER yanshi
ENV PORT=3000
ENV RUST_LOG=info
ENV YANSHI_PRICING_TABLE=/etc/yanshi/pricing.yml
ENV YANSHI_UI_DIR=/usr/share/yanshi/ui
EXPOSE 3000

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/yanshi"]
