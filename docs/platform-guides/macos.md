# macOS — Platform Guide

BetBlocker on macOS uses a Network Extension for DNS filtering and runs as a launchd daemon.

**Supported versions:** macOS 13 Ventura and later

---

## Installation

### Requirements

- macOS 13 Ventura or later
- Administrator account
- Apple Silicon or Intel x86-64

### Install via pkg

1. Download `BetBlocker-macOS.pkg` from your BetBlocker server's download page or betblocker.com
2. Double-click the package and follow the installer
3. macOS will prompt for your administrator password to install the system extension
4. On the **Server Configuration** screen:
   - **Server URL:** your server address (e.g. `https://betblocker.example.com`)
   - **Enrollment Token:** paste the token from the web dashboard
5. Click **Install**

### Approving the Network Extension (required)

After installation, macOS displays a notification:

> System Extension Blocked — A program tried to install a new system extension.

This approval is required for BetBlocker to function. Without it, DNS filtering will not work.

**To approve:**

1. Open **System Settings > Privacy & Security**
2. Scroll to the **Security** section
3. Next to "BetBlocker Network Extension was blocked", click **Allow**
4. Enter your administrator password
5. Reboot when prompted

After reboot, the Network Extension loads and blocking begins automatically.

> **Note for managed Macs:** If your Mac is enrolled in MDM (corporate or school management), your IT administrator may need to pre-approve the BetBlocker system extension via a configuration profile. See the MDM deployment section below.

---

## What the Agent Does

Two processes run continuously after installation:

| Process | Launched by | Account |
|---|---|---|
| `io.betblocker.agent` | launchd (system) | root |
| `io.betblocker.watchdog` | launchd (system) | root |

The **Network Extension** (`io.betblocker.networkextension`) runs in its own extension process managed by the OS — it cannot be stopped without removing the extension or rebooting.

The agent configures itself as the device's **DNS proxy** via the Network Extension's `NEDNSProxyProvider` API. All DNS queries from all apps on the device route through the agent's local resolver before reaching the network.

### LaunchDaemon plist locations

```
/Library/LaunchDaemons/io.betblocker.agent.plist
/Library/LaunchDaemons/io.betblocker.watchdog.plist
```

These are owned by root and locked from modification.

### Configuration and logs

```
/Library/Application Support/BetBlocker/   — configuration and blocklist cache
/Library/Logs/BetBlocker/                  — log files
```

---

## System Preferences Configuration

BetBlocker requires no manual System Settings configuration beyond the initial extension approval. However, the following settings are relevant if you are troubleshooting:

**Privacy & Security > Network Extensions:** BetBlocker should appear here as enabled. If it shows disabled, re-approve it.

**VPN & Filters (older macOS):** On macOS 14 and earlier, DNS filters appear under **System Settings > VPN > DNS**. BetBlocker will appear in this list. Do not manually disable it.

---

## MDM Deployment

For deploying to managed Macs via MDM (Jamf, Kandji, Mosyle, etc.):

### System Extension pre-approval payload

```xml
<dict>
    <key>PayloadType</key>
    <string>com.apple.system-extension-policy</string>
    <key>AllowedSystemExtensions</key>
    <dict>
        <key>io.betblocker</key>
        <array>
            <string>io.betblocker.networkextension</string>
        </array>
    </dict>
    <key>AllowedTeamIdentifiers</key>
    <array>
        <string>BETBLOCKER_TEAM_ID</string>
    </array>
</dict>
```

Replace `BETBLOCKER_TEAM_ID` with the BetBlocker code signing team identifier from the release notes.

### Silent installation

```bash
# Install pkg silently
sudo installer -pkg BetBlocker-macOS.pkg -target /

# Configure and enroll
sudo /Library/BetBlocker/betblocker-agent enroll \
  --server https://betblocker.example.com \
  --token tok_...
```

---

## Updating

Updates are applied automatically when the agent detects a new version on heartbeat. The update process:

1. Downloads and verifies the new `.pkg`
2. Installs in the background using `installer`
3. Restarts the agent daemons
4. If the Network Extension version changes, macOS may prompt the user to approve the new extension version — this requires the same approval flow as initial installation

For manual updates, download the new `.pkg` and run it.

---

## Uninstallation

Standard drag-to-Trash removal of the BetBlocker app does not uninstall the system components. Use the official uninstaller.

Uninstallation requires an approved unenrollment:

1. Complete the unenrollment process in the web dashboard
2. Run the uninstaller:
   ```bash
   sudo /Library/BetBlocker/uninstall.sh
   ```
3. Reboot when prompted (required to fully unload the Network Extension)

---

## Troubleshooting

### Network Extension not loading

```bash
# Check extension status
systemextensionsctl list | grep betblocker
# Expected: [activated enabled] io.betblocker.networkextension

# If status is [activated waiting for user]
# Go to System Settings > Privacy & Security and approve
```

### Agent not running

```bash
# Check launchd status
sudo launchctl list | grep betblocker

# View recent logs
log show --predicate 'subsystem == "io.betblocker"' --last 1h

# Or check log files directly
tail -f /Library/Logs/BetBlocker/agent.log
```

Start manually if needed (should not be necessary in normal operation):
```bash
sudo launchctl start io.betblocker.agent
```

### DNS not being filtered

```bash
# Verify BetBlocker is the active DNS proxy
scutil --dns | grep -A5 "resolver #1"
# Should show nameserver[0]: 127.0.0.53

# Test a block
nslookup gamblingsite.com 127.0.0.53
# Expected: NXDOMAIN
```

If `127.0.0.53` is not the resolver, the Network Extension may not be active. Check extension status (above) and re-approve if needed.

### "This Mac is managed" — conflicts with corporate MDM

If your Mac is managed by your employer, the corporate MDM may have DNS filters of its own that conflict with BetBlocker. In this case:

- Contact your IT department to allowlist BetBlocker's system extension
- Or use BetBlocker on a personal (unmanaged) Mac

BetBlocker cannot function correctly when a conflicting DNS filter is already active.

### Collecting a diagnostic bundle

```bash
sudo /Library/BetBlocker/betblocker-agent diagnostics --output ~/Desktop/bb-diag.zip
```

The bundle contains logs and configuration (secrets redacted). No browsing history or personal data is included.
