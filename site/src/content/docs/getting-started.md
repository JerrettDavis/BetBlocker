---
title: Getting Started
description: Install BetBlocker and enroll your first device
---


**Time to complete:** ~15 minutes (self-hosted) / ~5 minutes (hosted)

BetBlocker blocks gambling sites and apps at the network level on enrolled devices. This guide gets you from zero to a protected device.

---

## What You'll Have at the End

- A running BetBlocker instance (self-hosted) or an account on betblocker.com (hosted)
- One enrolled and protected device
- Optionally: an accountability partner linked to your account

---

## Prerequisites

### Option A — Self-hosted (Docker)

| Requirement | Minimum |
|---|---|
| Docker | 24+ |
| Docker Compose | v2 (bundled with Docker Desktop) |
| Open ports | 80 (web), 8443 (API) |
| RAM | 1 GB |
| Disk | 5 GB |

### Option B — Self-hosted (build from source)

| Requirement | Version |
|---|---|
| Rust toolchain | See `rust-toolchain.toml` |
| Node.js | 20+ |
| PostgreSQL | 16 with TimescaleDB extension |
| Redis | 7+ |

### Option C — Hosted (betblocker.com)

No server required. Create an account and skip to [Enrolling Your First Device](#4-enrolling-your-first-device).

---

## 1. Deploy the Server (Self-Hosted)

```bash
# Clone or download the release
git clone https://github.com/JerrettDavis/BetBlocker.git
cd betblocker/deploy

# Configure
cp .env.example .env
# Edit .env — at minimum set DB_PASSWORD and BETBLOCKER_EXTERNAL_URL
nano .env

# Pull images and start
docker compose up -d

# Verify all five containers are healthy
docker compose ps
```

Expected output — all services should show `healthy`:

```
NAME                    STATUS
betblocker-api-1        Up (healthy)
betblocker-worker-1     Up (healthy)
betblocker-web-1        Up (healthy)
betblocker-db-1         Up (healthy)
betblocker-cache-1      Up (healthy)
```

If a service is unhealthy, check its logs: `docker compose logs api`

---

## 2. First-Time Setup

Run the setup wizard once after first boot:

```bash
docker compose exec api /betblocker-api setup
```

Alternatively, run the bundled helper script:

```bash
../scripts/setup.sh
```

The setup wizard will:

1. Run database migrations
2. Generate cryptographic keys (JWT signing, blocklist signing, device CA)
3. Prompt you to create the first admin account
4. Perform an initial sync from the community blocklist feed (`https://feed.betblocker.org/v1`)

**Save the admin credentials securely.** They are not recoverable without direct database access.

### Verify the blocklist loaded

Open `https://your-server:8443/v1/blocklist/version` in a browser or with curl:

```bash
curl -s https://your-server:8443/v1/blocklist/version | jq .
# {"version": 1, "entry_count": 12345, "updated_at": "..."}
```

---

## 3. Configure the Blocklist (Admin)

Log in to the web dashboard at `http://your-server` (or `https://betblocker.com` for hosted).

Navigate to **Admin > Blocklist** to:

- Review what domains are currently blocked
- Add domains manually if needed
- Enable or disable the community feed sync

The community feed syncs automatically. The background worker refreshes it on a schedule — no manual action required after initial setup.

---

## 4. Enrolling Your First Device

### Step 1 — Create an enrollment token

From the web dashboard:

1. Go to **Devices > New Enrollment**
2. Choose your **enrollment tier**:
   - **Self** — you control unenrollment (with a configurable time delay)
   - **Partner** — an accountability partner must approve unenrollment
3. Copy the enrollment token or QR code

### Step 2 — Install the agent

Download the agent for your platform from the dashboard or your server's download page:

| Platform | Installer |
|---|---|
| Windows | `.msi` installer |
| macOS | `.pkg` installer |
| Linux | `systemd` package (`.deb` / `.rpm`) |
| Android | APK via Play Store or sideload |
| iOS | App Store (requires MDM for Authority tier) |

See the [platform guides](platform-guides/) for platform-specific installation steps.

### Step 3 — Enroll the device

During agent installation, enter:

- **Server URL:** your `BETBLOCKER_EXTERNAL_URL` (or `https://betblocker.com` for hosted)
- **Enrollment token:** from Step 1

The agent will:
1. Register the device with the server
2. Download the blocklist
3. Install itself as a system service
4. Begin blocking immediately

### Step 4 — Confirm enrollment

In the dashboard, the device should appear under **Devices** with status **Active** and a recent heartbeat timestamp within 60 seconds.

Test blocking works:

```
# On the enrolled device, try resolving a known gambling domain
nslookup betway.com
# Should return NXDOMAIN or the interstitial IP
```

---

## 5. Inviting an Accountability Partner

An accountability partner can see your device status, receive tamper alerts, and must approve any unenrollment request (if you enrolled under the Partner tier).

### From the dashboard

1. Go to **Partners > Invite Partner**
2. Enter their email address
3. They receive an invitation email with a link to create an account or log in

### What partners can see

| Item | Partner can see |
|---|---|
| Device online/offline status | Yes |
| Last heartbeat timestamp | Yes |
| Block attempt count (aggregated) | Yes |
| Individual blocked domains | Only with your explicit consent |
| Your browsing history | Never |

Partners cannot modify your enrollment configuration — they can only approve or deny unenrollment requests.

---

## Next Steps

- [Self-Hosting Guide](self-hosting.md) — production hardening, reverse proxy, TLS, backups
- [Platform Guides](platform-guides/) — platform-specific installation details
- [API Reference](api-reference.md) — integrate BetBlocker with your own tools
- [Architecture](architecture.md) — understand how the system works
