# Build the static SPA first so Rust can embed its production files.
FROM node:22-bookworm-slim AS frontend-builder
WORKDIR /app/frontend

COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml frontend/.npmrc ./
RUN corepack enable && pnpm install --frozen-lockfile

COPY frontend/tsconfig.json frontend/vite.config.ts ./
COPY frontend/src/ ./src/
COPY frontend/static/ ./static/
RUN pnpm build

# Compile dependencies in their own layer. Application and frontend changes then
# only rebuild the ocstats crate rather than its entire dependency graph.
FROM rust:1.88-bookworm AS rust-deps
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src target/release/.fingerprint/ocstats-* \
    && rm -f target/release/ocstats target/release/deps/ocstats-*

FROM rust-deps AS rust-builder

COPY src/ ./src/
COPY pricing.yaml ./
COPY --from=frontend-builder /app/frontend/build ./frontend/build
RUN cargo build --release --locked

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
