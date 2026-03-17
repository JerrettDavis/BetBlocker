---
title: Linux Guide
description: Install and configure BetBlocker on Linux
---


BetBlocker on Linux runs as a systemd service and uses iptables/nftables for network-level DNS redirection.

**Supported distributions:** Ubuntu 22.04+, Debian 12+, Fedora 38+, RHEL/AlmaLinux 9+

---

## Installation

### Requirements

- systemd (v245+)
- iptables or nftables
- x86-64 or aarch64 architecture
- Root or sudo access for installation

### Install via package manager

**Debian / Ubuntu (`.deb`):**

```bash
# Download the .deb from your server or betblocker.com
sudo apt install ./betblocker_1.2.3_amd64.deb
```

**Fedora / RHEL / AlmaLinux (`.rpm`):**

```bash
sudo dnf install ./betblocker-1.2.3.x86_64.rpm
```

The package installs the agent binary, systemd unit files, and an AppArmor/SELinux policy profile. It does not enroll the device.

### Enroll the device

After installing the package, enroll:

```bash
sudo betblocker-agent enroll \
  --server https://betblocker.example.com \
  --token tok_...
```

The enrollment command:
1. Registers the device with the server
2. Downloads the blocklist
3. Starts and enables the systemd service

Blocking begins immediately after enrollment.

### Verify

```bash
systemctl status betblocker-agent
# Expected: active (running)

# Test a block
nslookup gamblingsite.com 127.0.0.53
# Expected: NXDOMAIN
```

---

## What the Service Does

### systemd units

| Unit | Description |
|---|---|
| `betblocker-agent.service` | Main agent process. Runs the local DNS resolver, blocklist matching, heartbeat, and event reporting. |
| `betblocker-watchdog.service` | Monitors and restarts the agent. Mutual supervision — the agent also monitors the watchdog. |

Both units are enabled at boot and run as `root` (required for binding port 53 and managing iptables rules).

### DNS redirection

The agent installs an iptables (or nftables) rule that redirects all outbound UDP/TCP port 53 traffic to `127.0.0.53`:

```
# iptables rule installed by agent
-t nat -A OUTPUT -p udp --dport 53 -j DNAT --to-destination 127.0.0.53:53
-t nat -A OUTPUT -p tcp --dport 53 -j DNAT --to-destination 127.0.0.53:53
```

These rules persist across reboots via a `betblocker-iptables.service` unit that restores them on startup. The rules are applied for all users on the system.

### File locations

```
/usr/bin/betblocker-agent          — agent binary
/etc/betblocker/                   — configuration directory
/var/lib/betblocker/               — blocklist cache, device cert, local event DB
/var/log/betblocker/               — log files
/lib/systemd/system/               — unit files
/etc/apparmor.d/usr.bin.betblocker — AppArmor profile (if AppArmor is active)
```

---

## AppArmor Profile Setup

On Ubuntu and Debian, AppArmor is active by default and BetBlocker ships with a profile.

### Check profile status

```bash
sudo aa-status | grep betblocker
# Expected: /usr/bin/betblocker-agent (enforce)
```

### If the profile is in complain mode instead of enforce

```bash
sudo aa-enforce /etc/apparmor.d/usr.bin.betblocker
sudo systemctl restart betblocker-agent
```

### If you need to customise the profile

The profile restricts file access to the directories listed above and network access to the configured server URL. If you are running BetBlocker in a non-standard configuration (e.g., custom installation paths), edit `/etc/apparmor.d/usr.bin.betblocker` and reload:

```bash
sudo apparmor_parser -r /etc/apparmor.d/usr.bin.betblocker
```

---

## SELinux Profile Setup

On Fedora, RHEL, and AlmaLinux, SELinux is active in enforcing mode. BetBlocker ships an SELinux policy module.

### Install the policy module

The RPM package installs the module automatically. To verify:

```bash
semodule -l | grep betblocker
# Expected: betblocker
```

### If the module is not installed

```bash
sudo semodule -i /usr/share/betblocker/betblocker.pp
sudo restorecon -rv /usr/bin/betblocker-agent /etc/betblocker /var/lib/betblocker
```

### Check for SELinux denials

```bash
sudo ausearch -m avc -ts recent | grep betblocker
sudo journalctl -t setroubleshoot | grep betblocker
```

If you see denials that prevent the agent from functioning, generate a local policy to allow them:

```bash
sudo ausearch -m avc -ts recent | audit2allow -M betblocker-local
sudo semodule -i betblocker-local.pp
```

Report any denials that appear to be bugs — the upstream policy should cover all required access.

---

## Updating

The agent updates automatically via the package manager when a new version is available — if you have automatic updates enabled. The agent also self-updates when signalled by the server on heartbeat.

Manual update:

```bash
# Debian / Ubuntu
sudo apt update && sudo apt install betblocker

# Fedora / RHEL
sudo dnf update betblocker
```

The systemd units restart automatically after package update. Blocking is interrupted for only a few seconds during restart.

---

## Uninstallation

Uninstallation requires an approved unenrollment first.

1. Complete the unenrollment process in the web dashboard
2. Remove the package:
   ```bash
   # Debian / Ubuntu
   sudo apt remove --purge betblocker

   # Fedora / RHEL
   sudo dnf remove betblocker
   ```

The purge/remove step cleans up iptables rules, removes the AppArmor/SELinux profile, and disables the systemd units.

---

## Troubleshooting

### Service not starting

```bash
# Check service status and recent logs
systemctl status betblocker-agent
journalctl -u betblocker-agent -n 50 --no-pager
```

Common causes:

- **Port 53 already in use:** `systemd-resolved` or another local DNS service may be listening on 53.

  ```bash
  ss -tulnp | grep :53
  ```

  If `systemd-resolved` is using port 53:
  ```bash
  # Check if DNSStubListener is active
  grep DNSStubListener /etc/systemd/resolved.conf
  # If not set or set to yes, disable it:
  echo "DNSStubListener=no" | sudo tee -a /etc/systemd/resolved.conf
  sudo systemctl restart systemd-resolved
  sudo systemctl restart betblocker-agent
  ```

- **Cannot connect to server:** check `BETBLOCKER_SERVER_URL` in `/etc/betblocker/config.toml` and test connectivity:
  ```bash
  curl -sv https://betblocker.example.com/health
  ```

- **SELinux or AppArmor denial:** see the relevant section above.

### DNS not being redirected

```bash
# Check iptables rules are present
sudo iptables -t nat -L OUTPUT | grep DNAT

# If missing, restore them
sudo systemctl restart betblocker-iptables
```

If you are using `nftables` without iptables compatibility layer:

```bash
sudo nft list ruleset | grep betblocker
```

The agent auto-detects whether iptables or nftables is in use during installation.

### Gambling site not blocked

```bash
# Check what DNS server the agent is using
cat /etc/betblocker/config.toml | grep upstream_dns

# Test the local resolver directly
dig @127.0.0.53 gamblingsite.com
# Expected: NXDOMAIN

# Check the blocklist is current
sudo betblocker-agent status
# Shows: blocklist version, last sync, entry count
```

### Multi-user systems

On shared Linux machines, BetBlocker blocks DNS for all users — there is no per-user bypass. If a user has sudo access, they could potentially modify iptables rules. For maximum protection on shared systems, the AppArmor/SELinux profiles restrict the agent binary from modification, and the enrollment tier should be set to `partner` or `authority` so that the accountability structure is external to the machine.

### Collecting a diagnostic bundle

```bash
sudo betblocker-agent diagnostics --output /tmp/bb-diag.tar.gz
```

The bundle contains logs and configuration (secrets redacted). Share it with support if you cannot resolve an issue from the steps above.
