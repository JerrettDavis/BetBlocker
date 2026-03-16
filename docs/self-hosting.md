# Self-Hosting BetBlocker

This guide covers everything needed to run BetBlocker on your own infrastructure. The self-hosted deployment is identical in functionality to the hosted platform except billing and the automated discovery pipeline are disabled.

---

## System Requirements

### Minimum (single user, personal use)

| Resource | Minimum |
|---|---|
| CPU | 1 vCPU |
| RAM | 1 GB |
| Disk | 5 GB |
| OS | Any Linux, macOS, or Windows with Docker |

### Recommended (small organisation, up to ~50 devices)

| Resource | Recommended |
|---|---|
| CPU | 2 vCPU |
| RAM | 2 GB |
| Disk | 20 GB SSD |
| OS | Ubuntu 22.04 LTS / Debian 12 |

### Network requirements

- Outbound HTTPS to `feed.betblocker.org` (community blocklist sync)
- Inbound on your chosen `API_PORT` (default `8443`) reachable by enrolled devices
- Inbound on `WEB_PORT` (default `80`) for the web dashboard

---

## Docker Deployment (Recommended)

### 1. Install Docker

```bash
# Ubuntu / Debian
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
# Log out and back in for group change to take effect
```

Verify: `docker compose version` — must be v2.x (`docker compose`, not `docker-compose`).

### 2. Get the deployment files

```bash
git clone https://github.com/JerrettDavis/BetBlocker.git
cd betblocker/deploy
```

Or download just the `deploy/` directory from a release archive if you prefer not to clone the full repo.

### 3. Configure environment

```bash
cp .env.example .env
```

Edit `.env`:

```ini
# REQUIRED: change this
DB_PASSWORD=use-a-long-random-string-here

# REQUIRED: must be reachable by enrolled devices
BETBLOCKER_EXTERNAL_URL=https://betblocker.example.com:8443

# Optional overrides (defaults shown)
# BETBLOCKER_VERSION=latest
# API_PORT=8443
# WEB_PORT=80
# LOG_LEVEL=info
# BETBLOCKER_COMMUNITY_FEED_URL=https://feed.betblocker.org/v1
```

### 4. Start services

```bash
docker compose up -d
docker compose ps   # wait for all services to be healthy
```

### 5. Run first-time setup

```bash
docker compose exec api /betblocker-api setup
```

This runs migrations, generates keys, and prompts for the initial admin account. Run it once. Running it again on an already-initialised instance is a no-op.

### 6. Access the dashboard

Open `http://your-server` (or `https://` if you have TLS configured — see below). Log in with the admin credentials created in step 5.

---

## Manual Deployment (Build from Source)

Use this path if you cannot use Docker or need to customise the build.

### Prerequisites

```bash
# Rust (version pinned in rust-toolchain.toml)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Node.js 20+
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt-get install -y nodejs

# PostgreSQL 16 + TimescaleDB
# See: https://docs.timescale.com/self-hosted/latest/install/
```

### Build

```bash
# API and worker
cargo build --release -p bb-api -p bb-worker

# Web dashboard
cd web && npm ci && npm run build
```

Binaries will be at `target/release/bb-api` and `target/release/bb-worker`.

### Run

```bash
# Set all required environment variables (see Environment Variables section)
export BB_DATABASE_URL="postgres://betblocker:password@localhost:5432/betblocker"
export BB_REDIS_URL="redis://localhost:6379"
export BB_JWT_PRIVATE_KEY_PATH="/etc/betblocker/jwt-signing.pem"
export BB_JWT_PUBLIC_KEY_PATH="/etc/betblocker/jwt-signing-pub.pem"

./target/release/bb-api serve &
./target/release/bb-worker &

# Web dashboard
cd web && npm start
```

For production, wrap each process in a systemd unit. See `deploy/systemd/` for example unit files.

---

## Database Setup

The Docker deployment handles this automatically. If deploying manually:

### Install TimescaleDB

TimescaleDB is a PostgreSQL extension — it runs as a standard PostgreSQL instance:

```bash
# After installing PostgreSQL 16 and the timescaledb package:
sudo -u postgres psql -c "CREATE USER betblocker WITH PASSWORD 'your-password';"
sudo -u postgres psql -c "CREATE DATABASE betblocker OWNER betblocker;"
sudo -u postgres psql -d betblocker -c "CREATE EXTENSION IF NOT EXISTS timescaledb;"
```

### Run migrations

```bash
./target/release/bb-api migrate
```

Migrations are embedded in the binary and run automatically via `setup` or can be run explicitly with `migrate`.

### TimescaleDB hypertables

The worker creates hypertable partitioning on the events table during first setup. No manual SQL is needed.

---

## Environment Variables Reference

All variables use the `BB_` prefix (e.g., `BB_DATABASE_URL`). In `docker-compose.yml` some variables are named without the prefix for Docker compatibility — the table below shows both forms where they differ.

| Variable | Required | Default | Description |
|---|---|---|---|
| `BB_DATABASE_URL` | Yes | — | PostgreSQL connection string |
| `BB_REDIS_URL` | No | `redis://localhost:6379` | Redis connection URL |
| `BB_HOST` | No | `0.0.0.0` | API bind address |
| `BB_PORT` | No | `3000` | API bind port (Docker exposes on 8443) |
| `BB_JWT_PRIVATE_KEY_PATH` | Yes | — | Path to Ed25519 private key PEM for JWT signing |
| `BB_JWT_PUBLIC_KEY_PATH` | Yes | — | Path to Ed25519 public key PEM for JWT verification |
| `BB_JWT_ACCESS_TOKEN_TTL_SECS` | No | `3600` | Access token lifetime in seconds |
| `BB_JWT_REFRESH_TOKEN_TTL_DAYS` | No | `30` | Refresh token lifetime in days |
| `BB_CORS_ALLOWED_ORIGINS` | No | `*` | Comma-separated allowed CORS origins |
| `BB_PUBLIC_BASE_URL` | No | — | Public URL used in generated links and QR codes |
| `BB_BILLING_ENABLED` | No | `false` | Enable Stripe billing endpoints (hosted only) |
| `BB_STRIPE_SECRET_KEY` | Conditional | — | Required when `BILLING_ENABLED=true` |
| `BB_STRIPE_WEBHOOK_SECRET` | Conditional | — | Required when `BILLING_ENABLED=true` |
| `BETBLOCKER_DEPLOYMENT` | No | — | Set to `self-hosted` to disable hosted-only features |
| `BETBLOCKER_COMMUNITY_FEED_URL` | No | `https://feed.betblocker.org/v1` | Blocklist feed URL |
| `BETBLOCKER_FEDERATED_REPORT_UPSTREAM` | No | — | Opt-in: upstream URL for federated report contribution |
| `BETBLOCKER_FEDERATED_REPORT_API_KEY` | No | — | API key for federated upstream |
| `LOG_LEVEL` | No | `info` | Log verbosity: `trace`, `debug`, `info`, `warn`, `error` |
| `BETBLOCKER_LOG_FORMAT` | No | `json` | Log format: `json` or `text` |

### Key files (mounted via Docker volumes)

| Path | Contents |
|---|---|
| `/keys/jwt-signing.key` | Ed25519 JWT signing key |
| `/keys/blocklist-signing.key` | Blocklist signing key |
| `/keys/root-ca.key` | Root CA private key |
| `/keys/device-ca.key` | Device certificate CA key |

Keys are generated by `bb-api setup` and stored in the `betblocker-keys` Docker volume. Back this volume up — losing it requires re-enrolling all devices.

---

## Reverse Proxy Setup (nginx)

Run nginx in front of the API if you want standard HTTPS on port 443 and to avoid exposing port 8443.

### nginx configuration

```nginx
# /etc/nginx/sites-available/betblocker

# Redirect HTTP to HTTPS
server {
    listen 80;
    server_name betblocker.example.com;
    return 301 https://$host$request_uri;
}

# API
server {
    listen 443 ssl http2;
    server_name betblocker.example.com;

    ssl_certificate     /etc/letsencrypt/live/betblocker.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/betblocker.example.com/privkey.pem;
    ssl_protocols       TLSv1.2 TLSv1.3;
    ssl_ciphers         ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;

    # Web dashboard
    location / {
        proxy_pass         http://127.0.0.1:80;
        proxy_set_header   Host $host;
        proxy_set_header   X-Real-IP $remote_addr;
        proxy_set_header   X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header   X-Forwarded-Proto $scheme;
    }

    # API
    location /v1/ {
        proxy_pass         http://127.0.0.1:8443;
        proxy_set_header   Host $host;
        proxy_set_header   X-Real-IP $remote_addr;
        proxy_set_header   X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header   X-Forwarded-Proto $scheme;
        proxy_read_timeout 30s;
        client_max_body_size 2M;
    }

    location /health {
        proxy_pass http://127.0.0.1:8443;
    }
}
```

```bash
sudo ln -s /etc/nginx/sites-available/betblocker /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx
```

After setting up the proxy, update `.env`:

```ini
BETBLOCKER_EXTERNAL_URL=https://betblocker.example.com
API_PORT=8443   # still needed for container binding
```

---

## TLS/SSL Certificates

### Let's Encrypt (recommended for internet-facing instances)

```bash
sudo apt install certbot python3-certbot-nginx
sudo certbot --nginx -d betblocker.example.com
```

Certbot will auto-renew via a systemd timer. Verify: `sudo certbot renew --dry-run`

### Self-signed certificate (LAN / internal use)

If your instance is not internet-facing:

```bash
openssl req -x509 -newkey ed25519 \
  -keyout betblocker-tls.key \
  -out betblocker-tls.crt \
  -days 3650 \
  -nodes \
  -subj "/CN=betblocker.local"
```

Enrolled devices must trust your self-signed CA. The agent allows configuring a custom CA certificate at enrollment time.

---

## Backup and Restore

### What to back up

| Data | Location | Priority |
|---|---|---|
| Database | `betblocker-db` Docker volume | Critical |
| Keys | `betblocker-keys` Docker volume | Critical — losing keys requires full re-enrollment |
| Data | `betblocker-data` Docker volume | Important |
| `.env` file | `deploy/.env` | Important |

### Database backup

```bash
# Dump
docker compose exec db pg_dump -U betblocker betblocker | gzip > backup-$(date +%Y%m%d).sql.gz

# Restore
gunzip -c backup-20260315.sql.gz | docker compose exec -T db psql -U betblocker betblocker
```

### Volume backup

```bash
# Backup keys volume (CRITICAL)
docker run --rm \
  -v betblocker_betblocker-keys:/keys:ro \
  -v $(pwd):/backup \
  alpine tar czf /backup/keys-backup-$(date +%Y%m%d).tar.gz -C /keys .
```

### Restore from backup

1. Stop services: `docker compose down`
2. Restore volumes from archives
3. Restore database from dump
4. Start services: `docker compose up -d`

---

## Updating to a New Version

```bash
cd deploy

# Pull new images
docker compose pull

# Restart with new images (zero-downtime if you have multiple API replicas)
docker compose up -d

# Migrations run automatically on API startup
# Check logs to confirm
docker compose logs api | grep -E "migration|error"
```

To pin a specific version, set `BETBLOCKER_VERSION=1.2.3` in `.env`.

---

## Monitoring and Health Checks

### Health endpoint

```bash
curl -s https://your-server/health | jq .
# {"status": "ok", "version": "1.2.3", "db": "ok", "cache": "ok"}
```

### Container health

All containers expose Docker health checks. Integrate with your monitoring tool:

```bash
# Quick status
docker compose ps

# JSON output for scripting
docker compose ps --format json
```

### Log aggregation

Logs are emitted as JSON to stdout (configurable via `BETBLOCKER_LOG_FORMAT`). Pipe to any log aggregator:

```bash
# Example: ship to a syslog target
docker compose logs -f api | logger -t betblocker-api

# Example: forward with Filebeat / Fluentd
# Point your agent at the Docker daemon's log driver output
```

### Key metrics to alert on

| Metric | Alert condition |
|---|---|
| Container restarts | > 2 in 5 minutes |
| API health endpoint | Non-`ok` response |
| Database disk | > 80% full |
| Worker last run | No successful run in 1 hour |
| Heartbeat failures | Spike in missed device heartbeats |

### Disk usage

The database grows as events accumulate. TimescaleDB's compression and retention policies reduce this significantly. Configure retention from **Admin > Analytics Settings** in the dashboard.

---

## Federated Reporting (Opt-In)

Self-hosted instances can contribute unknown domain reports back to the central BetBlocker blocklist. This strengthens the community feed for everyone.

To opt in, add to `.env`:

```ini
BETBLOCKER_FEDERATED_REPORT_UPSTREAM=https://api.betblocker.org/v1/reports
BETBLOCKER_FEDERATED_REPORT_API_KEY=your-key-from-betblocker.com
```

**Privacy guarantee:** only blocked/flagged domain metadata is sent — never full browsing history, never user-identifying information, never device identifiers. Source IP addresses are stripped before federated report processing.
