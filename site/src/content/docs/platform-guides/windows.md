---
title: Windows Guide
description: Install and configure BetBlocker on Windows
---


BetBlocker on Windows runs as a Windows Service and uses the Windows Filtering Platform (WFP) for network-level blocking.

**Supported versions:** Windows 10 (1903+), Windows 11

---

## Installation

### Requirements

- Windows 10 version 1903 or later, or Windows 11
- Administrator account for installation
- .NET Runtime is not required — the agent is a self-contained binary

### Install via MSI

1. Download `BetBlocker-Setup-x64.msi` from your BetBlocker server's download page or from betblocker.com
2. Right-click the MSI and select **Run as administrator** (or double-click — the installer will prompt for elevation)
3. Follow the installation wizard
4. On the **Server Configuration** screen:
   - **Server URL:** your server address (e.g. `https://betblocker.example.com`)
   - **Enrollment Token:** paste the token from the web dashboard
5. Click **Install** — the wizard installs the service, enrolls the device, and downloads the blocklist
6. Installation completes in under a minute

The installer does not require a reboot. Blocking begins immediately.

### Silent installation (enterprise / bulk deployment)

```powershell
msiexec /i BetBlocker-Setup-x64.msi /quiet /norestart `
  BETBLOCKER_SERVER_URL="https://betblocker.example.com" `
  BETBLOCKER_TOKEN="tok_..." `
  BETBLOCKER_LOG_LEVEL="info"
```

---

## What the Service Does

After installation, three components run at all times:

| Component | Type | Account |
|---|---|---|
| `BetBlockerAgent` | Windows Service | `SYSTEM` |
| `BetBlockerWatchdog` | Windows Service | `SYSTEM` |
| WFP callout driver | Kernel driver | — |

**`BetBlockerAgent`** — the main agent process. Runs the local DNS resolver on `127.0.0.53`, matches queries against the blocklist, reports events, and sends heartbeats.

**`BetBlockerWatchdog`** — monitors the agent service and restarts it if it crashes. The agent reciprocally monitors the watchdog.

**WFP callout driver** — operates at the Windows network stack level. Enforces DNS query redirection through the local resolver for all processes, including apps with hardcoded DNS settings. The WFP rules persist even if the agent service is stopped — this is by design. Removing the driver requires either the uninstaller or an authorised unenrollment.

### DNS configuration

The installer sets the loopback DNS resolver (`127.0.0.53`) as the primary DNS server for all active network adapters. The agent then acts as the device's DNS resolver and handles all upstream queries.

### Registry protection

The agent's configuration and enrollment credentials are stored encrypted in `HKLM\SOFTWARE\BetBlocker` with ACLs that prevent modification by non-SYSTEM accounts.

---

## Updating

The agent checks for updates on each heartbeat. When a new version is available:

1. The server signals `command: update` in the heartbeat response
2. The agent downloads and verifies the new installer
3. The update applies automatically in the background
4. Services restart with no user interaction required

For manual updates, download the new MSI and run it — the installer handles in-place upgrade.

---

## Uninstallation

Uninstallation requires either:
- A completed (approved) unenrollment request, **or**
- Admin credentials on the machine (for self-tier enrollments after the time delay has elapsed)

Standard removal via **Settings > Apps** will fail with a protected installation message if an active enrollment is present.

For authorised removal:
1. Complete the unenrollment process in the web dashboard first
2. Then use **Settings > Apps > BetBlocker > Uninstall**, or run `msiexec /x BetBlocker-Setup-x64.msi`

---

## Troubleshooting

### Service not starting

```powershell
# Check service status
Get-Service BetBlockerAgent, BetBlockerWatchdog

# Check Windows Event Log for error detail
Get-EventLog -LogName Application -Source BetBlocker -Newest 20

# Check agent log file
Get-Content "$env:ProgramData\BetBlocker\logs\agent.log" -Tail 50
```

Common causes:
- **Database connection error:** the agent cannot reach the server. Check `BETBLOCKER_SERVER_URL` and network connectivity.
- **Certificate error:** the device certificate has expired or been revoked. Re-enroll the device.
- **Permission error:** the service account does not have access to the config directory. Check `%ProgramData%\BetBlocker` permissions.

### DNS not being redirected

```powershell
# Verify DNS server is set to loopback
Get-DnsClientServerAddress | Select-Object InterfaceAlias, ServerAddresses

# Verify the local resolver is listening
netstat -an | findstr ":53"

# Test a block
nslookup gamblingsite.com 127.0.0.53
# Expected: Non-existent domain (NXDOMAIN)
```

If the local resolver is not listening, the agent service may not be running. Start it:
```powershell
Start-Service BetBlockerAgent
```

If DNS is still not redirected after the service starts, the WFP driver may not be loaded:
```powershell
# Check driver status
sc query BetBlockerWFP
# Expected: STATE: 4 RUNNING
```

### Gambling site not blocked

1. Check the blocklist version: open `https://your-server/v1/blocklist/version` and compare to the version in `%ProgramData%\BetBlocker\status.json`
2. If the local version is behind, force a sync: right-click the BetBlocker tray icon > **Check for updates**
3. If the domain is not in the blocklist, submit it via the dashboard: **Admin > Blocklist > Add domain**

### Tamper alert fired incorrectly

If you made a legitimate change (new network adapter, VPN for work) that triggered a tamper alert:
1. Log in to the web dashboard
2. Go to **Devices > [your device] > Alerts**
3. Mark the alert as acknowledged
4. If using a work VPN regularly, add it to the bypass allowlist in your enrollment configuration

### Collecting a diagnostic bundle

```powershell
# Run as Administrator
& "$env:ProgramFiles\BetBlocker\betblocker-agent.exe" diagnostics --output C:\Temp\bb-diag.zip
```

Share `bb-diag.zip` with support. It contains logs, configuration (with secrets redacted), and service status — no personal browsing data.
