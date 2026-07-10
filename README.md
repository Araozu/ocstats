# OC Stats

OC Stats imports and displays local OpenCode usage data.

## Build and run

Build the Svelte single-page application before compiling the Rust binary. The frontend build is embedded in the executable, so Node.js is not needed at runtime.

```bash
pnpm --dir frontend build
cargo build --release
./target/release/ocstats serve --port 4117
```

Open `http://127.0.0.1:4117`. The binary serves both the dashboard and its `/api` endpoints.
