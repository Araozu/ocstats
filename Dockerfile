# Build the static SPA first so Rust can embed its production files.
FROM node:22-bookworm-slim AS frontend-builder
WORKDIR /app/frontend

COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
RUN corepack enable && pnpm install --frozen-lockfile

COPY frontend/ ./
RUN pnpm build

FROM rust:1.88-bookworm AS rust-builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY pricing.yaml ./
COPY --from=frontend-builder /app/frontend/build ./frontend/build
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /var/lib/ocstats

COPY --from=rust-builder /app/target/release/ocstats /usr/local/bin/ocstats

ENV XDG_DATA_HOME=/var/lib/ocstats
ENV OCSTATS_PORT=4117
ENV OPENCODE_BASE_PATH=/opencode

VOLUME ["/var/lib/ocstats"]
EXPOSE 4117

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["/usr/local/bin/ocstats", "healthcheck"]

CMD ["/usr/local/bin/ocstats", "serve"]
