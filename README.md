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

## Container configuration

The container receives its configuration from environment variables:

| Variable | Default | Description |
| --- | --- | --- |
| `OCSTATS_PORT` | `4117` | HTTP server port. A `serve --port` argument takes precedence. |
| `OPENCODE_BASE_PATH` | OpenCode's standard XDG data directory | Directory containing `opencode.db`. |

The image stores OC Stats' imported analytics database in `/var/lib/ocstats`, separately from the read-only OpenCode data mount.

Build the image with:

```bash
docker build -t ocstats .
```

`compose.yml` configures Traefik for `oc.araoz.dev` and mounts the OpenCode data from `./data/opencode/share`. Start it with `docker compose up --build -d` after creating the external `proxy` network.

```yaml
services:
  ocstats:
    build: .
    ports:
      - "4117:4117"
    environment:
      OCSTATS_PORT: "4117"
      OPENCODE_BASE_PATH: /opencode
    volumes:
      - ${HOME}/.local/share/opencode:/opencode:ro
      - ocstats-data:/var/lib/ocstats

volumes:
  ocstats-data:
```
