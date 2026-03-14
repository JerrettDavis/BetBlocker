# BetBlocker Network Extension (Swift stub)

This directory contains the configuration and documentation scaffold for the
BetBlocker macOS Network Extension.  The actual implementation requires Xcode
and must be written in Swift (or Objective-C); it cannot be compiled from this
Rust monorepo.

---

## Overview

The Network Extension provides transparent DNS filtering at the kernel level
via Apple's `NetworkExtension.framework` (`NEDNSProxyProvider`).  It runs as a
System Extension loaded by the macOS kernel, entirely separate from the
`bb-agent-macos` daemon.  The two processes communicate over XPC using the
Rust `bb-shim-macos::xpc` module on the agent side and a matching Swift XPC
listener on the extension side.

```
┌─────────────────────────────┐           XPC            ┌──────────────────────────────────┐
│  bb-agent-macos (Rust daemon)│ ◄────────────────────── │ BetBlockerNetworkExtension (Swift)│
│  bb_shim_macos::xpc          │   JSON over Mach port   │  NEDNSProxyProvider subclass      │
└─────────────────────────────┘                          └──────────────────────────────────┘
```

---

## Swift Implementation Required

### 1. Create an Xcode project

1. Open Xcode → File → New → Project → macOS → System Extension.
2. Set the bundle identifier to `com.betblocker.networkextension`.
3. Add the **Network Extension** capability.
4. Enable the **DNS Proxy** extension type.

### 2. Implement `NEDNSProxyProvider`

Create `DNSProxyProvider.swift`:

```swift
import NetworkExtension
import os.log

class DNSProxyProvider: NEDNSProxyProvider {

    private let log = Logger(subsystem: "com.betblocker", category: "dns-proxy")
    private var blocklist: Set<String> = []

    override func startProxy(options: [String: Any]?, completionHandler: @escaping (Error?) -> Void) {
        log.info("BetBlocker DNS proxy starting")
        XPCListener.shared.start(provider: self)
        completionHandler(nil)
    }

    override func stopProxy(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        log.info("BetBlocker DNS proxy stopping, reason: \(reason.rawValue)")
        XPCListener.shared.stop()
        completionHandler()
    }

    override func handleNewFlow(_ flow: NEAppProxyFlow) -> Bool {
        // Not used for DNS proxy; flow handling happens in handleNewUDPFlow.
        return false
    }

    override func handleNewUDPFlow(_ flow: NEAppProxyUDPFlow, initialRemoteEndpoint remoteEndpoint: NWEndpoint) -> Bool {
        // Intercept DNS queries (port 53) and apply blocklist.
        // Return false to pass through non-DNS traffic.
        guard let endpoint = remoteEndpoint as? NWHostEndpoint, endpoint.port == "53" else {
            return false
        }
        // TODO: parse DNS query, check against blocklist, synthesise NXDOMAIN if blocked.
        return true
    }

    // Called by the XPC listener when the Rust agent pushes an updated blocklist.
    func updateBlocklist(_ domains: [String]) {
        blocklist = Set(domains)
        log.info("Blocklist updated: \(domains.count) domains")
    }
}
```

### 3. XPC protocol

Define a shared Swift protocol that mirrors the Rust `XpcMessage` / `XpcResponse`
enums.  The Rust agent serialises messages as JSON strings placed in an XPC
dictionary under the key `"payload"`; the Swift listener must deserialise and
dispatch them.

```swift
// XPCListener.swift
import Foundation

class XPCListener: NSObject, NSXPCListenerDelegate {
    static let shared = XPCListener()
    private var listener: NSXPCListener?
    weak var provider: DNSProxyProvider?

    func start(provider: DNSProxyProvider) {
        self.provider = provider
        let listener = NSXPCListener(machServiceName: "com.betblocker.networkextension")
        listener.delegate = self
        listener.resume()
        self.listener = listener
    }

    func stop() {
        listener?.invalidate()
        listener = nil
    }

    func listener(_ listener: NSXPCListener, shouldAcceptNewConnection conn: NSXPCConnection) -> Bool {
        conn.exportedInterface = NSXPCInterface(with: BetBlockerXPCProtocol.self)
        conn.exportedObject = XPCHandler(provider: provider)
        conn.resume()
        return true
    }
}

@objc protocol BetBlockerXPCProtocol {
    func sendMessage(_ payload: String, reply: @escaping (String) -> Void)
}

class XPCHandler: NSObject, BetBlockerXPCProtocol {
    weak var provider: DNSProxyProvider?
    init(provider: DNSProxyProvider?) { self.provider = provider }

    func sendMessage(_ payload: String, reply: @escaping (String) -> Void) {
        // Decode the JSON XpcMessage sent by the Rust agent and respond.
        guard let data = payload.data(using: .utf8),
              let msg = try? JSONDecoder().decode(XpcMessage.self, from: data) else {
            reply(encode(XpcResponse.error("invalid payload")))
            return
        }
        switch msg {
        case .getStatus:
            reply(encode(XpcResponse.status(active: true, blockedCount: UInt64(provider?.blocklist.count ?? 0))))
        case .enableFiltering:
            reply(encode(XpcResponse.ok))
        case .disableFiltering:
            reply(encode(XpcResponse.ok))
        case .updateBlocklist(let domains):
            provider?.updateBlocklist(domains)
            reply(encode(XpcResponse.ok))
        }
    }

    private func encode(_ resp: XpcResponse) -> String {
        let data = try? JSONEncoder().encode(resp)
        return data.flatMap { String(data: $0, encoding: .utf8) } ?? "{\"Ok\":null}"
    }
}
```

### 4. Entitlements

The Network Extension bundle **must** have the following entitlements in its
`.entitlements` file (set via Xcode Signing & Capabilities):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- Required for all System Extensions -->
    <key>com.apple.developer.system-extension.install</key>
    <true/>

    <!-- Required for NEDNSProxyProvider -->
    <key>com.apple.developer.networking.networkextension</key>
    <array>
        <string>dns-proxy</string>
    </array>

    <!-- Required for XPC Mach service registration -->
    <key>com.apple.security.application-groups</key>
    <array>
        <string>TEAMID.com.betblocker</string>
    </array>
</dict>
</plist>
```

The **configuring app** (or `bb-agent-macos` helper tool) needs:

```xml
<key>com.apple.developer.system-extension.install</key>
<true/>
<key>com.apple.developer.networking.networkextension</key>
<array>
    <string>dns-proxy-provider</string>
</array>
```

### 5. System Extension activation

The extension is activated at runtime via `OSSystemExtensionManager`:

```swift
import SystemExtensions

func activateExtension() {
    let req = OSSystemExtensionRequest.activationRequest(
        forExtensionWithIdentifier: "com.betblocker.networkextension",
        queue: .main
    )
    req.delegate = self
    OSSystemExtensionManager.shared.submitRequest(req)
}
```

This must be called from a **notarised, code-signed** application bundle.
It cannot be called from a bare command-line tool.

---

## XPC Message Protocol (JSON)

The Rust `XpcMessage` and `XpcResponse` types are serialised as JSON with
`serde_json` (see `crates/bb-shim-macos/src/xpc.rs`).  The wire format for
each variant is:

### XpcMessage

| Rust variant                      | JSON wire format                              |
|-----------------------------------|-----------------------------------------------|
| `XpcMessage::GetStatus`           | `"GetStatus"`                                 |
| `XpcMessage::EnableFiltering`     | `"EnableFiltering"`                           |
| `XpcMessage::DisableFiltering`    | `"DisableFiltering"`                          |
| `XpcMessage::UpdateBlocklist(v)`  | `{"UpdateBlocklist":["a.com","b.com",...]}`   |

### XpcResponse

| Rust variant                               | JSON wire format                                  |
|--------------------------------------------|---------------------------------------------------|
| `XpcResponse::Ok`                          | `"Ok"`                                            |
| `XpcResponse::Error("msg")`                | `{"Error":"msg"}`                                 |
| `XpcResponse::Status{active, blocked_count}` | `{"Status":{"active":true,"blocked_count":42}}` |

---

## Build Requirements

- macOS 12.0 or later
- Xcode 14 or later
- Apple Developer Program membership (for code signing + notarisation)
- Provisioning profile with Network Extension entitlement

---

## Files in this directory

| File         | Purpose                                                      |
|--------------|--------------------------------------------------------------|
| `Info.plist` | Bundle metadata required by the NetworkExtension framework   |
| `README.md`  | This document                                                |

The Swift source files, Xcode project, and entitlements files must be created
separately using Xcode.
