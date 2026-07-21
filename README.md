# OC Stats

OC Stats is a self-hosted dashboard for exploring [OpenCode](https://opencode.ai/) usage. It imports OpenCode's local SQLite data into a separate analytics database and shows costs, tokens, models, projects, sessions, and individual assistant turns.

OC Stats only reads the OpenCode database. It does not call model providers, modify OpenCode data, or read OpenCode credentials.

## Quick start

The published image is available as [`araozu/ocstats`](https://hub.docker.com/r/araozu/ocstats).

1. Copy the example configuration:

   ```bash
   cp .env.example .env
   ```

2. Set `OCSTATS_PASSWORD` and the absolute path to your OpenCode data directory in `.env`.

3. Start OC Stats:

   ```bash
   docker compose up -d
   ```

4. Open `https://stats.example.com` through a reverse proxy. For direct local HTTP access, set `OCSTATS_COOKIE_SECURE=false` and open `http://127.0.0.1:4117`.

5. Sign in and select **Import** to load the latest OpenCode data. Imports insert new records and update existing ones, so they can be run again whenever you want to refresh the dashboard.

The default Compose configuration binds only to localhost. This avoids exposing the service directly while still allowing a reverse proxy on the host to reach it.

## Configuration

| Variable | Image default | Description |
| --- | --- | --- |
| `OCSTATS_PASSWORD` | None | Required, non-empty dashboard password. Use a long random value, for example from `openssl rand -base64 32`. |
| `OCSTATS_PORT` | `4117` | Port inside the container. The supplied Compose file deliberately keeps this at `4117`; use `OCSTATS_HTTP_PORT` to change the published host port. |
| `OCSTATS_COOKIE_SECURE` | `true` | Controls the session cookie's `Secure` flag. Keep `true` with HTTPS; use the exact value `false` only for local HTTP. |
| `OPENCODE_BASE_PATH` | `/opencode` | Directory containing the source OpenCode database. The Compose file mounts your host directory here. |
| `OPENCODE_DB` | `opencode.db` | Source database filename. Relative paths resolve under `OPENCODE_BASE_PATH`; absolute paths are also accepted. Useful for non-stable OpenCode channels. |
| `XDG_DATA_HOME` | `/var/lib/ocstats` | Writable application data root. The analytics database is stored at `/var/lib/ocstats/ocstats/analytics.db`. |
| `OCSTATS_PRICING_FILE` | Embedded catalog | Optional path to a replacement model-pricing YAML file. Mount the file into the container when using this setting. |

`OCSTATS_HTTP_PORT` and `OPENCODE_DATA_DIR` are Compose variables, not application variables. They configure the host port and source bind mount respectively. The supplied Compose file also passes `OPENCODE_DB`; add any other optional application variables to its `environment` section explicitly.

To supply a custom pricing catalog, add both the environment variable and a read-only file mount:

```yaml
services:
  ocstats:
    environment:
      OCSTATS_PRICING_FILE: /config/pricing.yaml
    volumes:
      - ./pricing.yaml:/config/pricing.yaml:ro
```

The login is stored in an HttpOnly, `SameSite=Strict` cookie. OC Stats should be served as one origin, and currently expects to run at the root of a hostname rather than below a URL path such as `/ocstats`.

## Storage

The container uses two distinct storage locations:

| Container path | Access | Purpose |
| --- | --- | --- |
| `/opencode` | Read-only bind mount | OpenCode's `opencode.db` and adjacent SQLite `-wal`/`-shm` files. |
| `/var/lib/ocstats` | Read-write named volume | OC Stats analytics data and pricing requests. |

Mount the entire OpenCode data directory, not only `opencode.db`. OpenCode uses SQLite WAL mode, so an active database can depend on adjacent `opencode.db-wal` and `opencode.db-shm` files. The source is opened read-only and is never modified by OC Stats.

The supplied Compose file uses the named volume `ocstats-data`. To use an easily browsable host directory instead, replace it with a bind mount:

```yaml
volumes:
  - ./data/ocstats:/var/lib/ocstats
```

Back up the analytics volume with the service stopped so SQLite is in a consistent state:

```bash
docker compose stop ocstats
docker run --rm --volumes-from ocstats -v "$PWD":/backup \
  alpine tar czf /backup/ocstats-backup.tar.gz -C /var/lib/ocstats .
docker compose start ocstats
```

`--volumes-from ocstats` uses the volume attached to the stopped OC Stats container, so the command does not depend on Compose's generated volume name.

Imports do not remove analytics records when the corresponding OpenCode records have been deleted. To rebuild from only the current source data, remove the OC Stats volume and import again. This permanently deletes the existing analytics database:

```bash
docker compose down --volumes
docker compose up -d
```

## Reverse proxies

Keep `OCSTATS_COOKIE_SECURE=true` for every HTTPS setup. Point a dedicated hostname at OC Stats; subpath hosting is not currently supported.

### Caddy

[`examples/caddy/Caddyfile`](examples/caddy/Caddyfile) is a minimal host-installed Caddy configuration with automatic HTTPS. Replace the hostname, install it as your Caddyfile, and reload Caddy.

```caddyfile
stats.example.com {
	reverse_proxy 127.0.0.1:4117
}
```

### Nginx

[`examples/nginx/ocstats.conf`](examples/nginx/ocstats.conf) includes HTTP-to-HTTPS redirection, TLS, and forwarded headers. Replace the hostname and certificate paths, then reload Nginx. The Compose service remains reachable at `127.0.0.1:4117`.

### Traefik

The Traefik override expects an existing external network called `proxy`, a `websecure` entrypoint, and a certificate resolver. Add these values to `.env`:

```dotenv
OCSTATS_HOSTNAME=stats.example.com
TRAEFIK_CERT_RESOLVER=letsencrypt
```

Then start both Compose files:

```bash
docker compose -f compose.yml -f examples/traefik/compose.yml up -d
```

If your Traefik network, entrypoint, or resolver has a different name, edit [`examples/traefik/compose.yml`](examples/traefik/compose.yml).

### Other proxies

Proxy the hostname's `/` path to `http://127.0.0.1:4117`, preserve the `Host` header, and send `X-Forwarded-Proto: https`. No WebSocket configuration is required. `GET /api/health` is available for external health checks.

## Health checks

The image includes a health check that verifies the configured OpenCode database exists and can be opened read-only. The same check is exposed at `GET /api/health` and returns HTTP `503` when the source database is inaccessible.

This specifically checks source-database access; it is not a comprehensive check of the analytics database or HTTP listener.

```bash
docker compose ps
curl --fail http://127.0.0.1:4117/api/health
docker compose logs -f ocstats
```

## Updating

```bash
docker compose pull
docker compose up -d
```

Pin a release tag instead of `latest` in `compose.yml` if you prefer controlled upgrades.

## Build from source

Requirements:

- Rust 1.88 or newer
- Node.js 22
- pnpm via Corepack

The frontend is embedded into the Rust executable at compile time:

```bash
corepack enable
pnpm --dir frontend install --frozen-lockfile
pnpm --dir frontend build
cargo build --release --locked
OCSTATS_PASSWORD=development-password \
OCSTATS_COOKIE_SECURE=false \
./target/release/ocstats serve --port 4117
```

To build the container locally instead of pulling Docker Hub:

```bash
docker build -t ocstats .
```

## Development

Run the project checks with:

```bash
cargo test --locked
pnpm --dir frontend check
pnpm --dir frontend test
```

Issues and pull requests are welcome. When reporting an import problem, include the OC Stats logs and OpenCode version, but do not attach your databases because they can contain project and conversation data.

## Security

- Do not expose port 4117 directly to the public internet; keep the localhost binding and place it behind HTTPS.
- Treat `OCSTATS_PASSWORD`, the OpenCode database, and the analytics volume as sensitive.
- Environment variables can be visible through container-management tooling. Restrict Docker access accordingly.
- The OpenCode database may contain project paths, prompts, and assistant output in addition to usage metadata.

## License

OC Stats is licensed under the [GNU General Public License version 2 only](LICENSE), identified by SPDX as `GPL-2.0-only`.
