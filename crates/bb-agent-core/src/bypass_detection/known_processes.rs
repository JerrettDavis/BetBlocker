/// Well-known VPN process names across platforms.
pub const VPN_PROCESS_NAMES: &[&str] = &[
    "openvpn",
    "wireguard-go",
    "wg-quick",
    "nordvpn",
    "nordlynx",
    "expressvpn",
    "expressvpnd",
    "mullvad-daemon",
    "mullvad-vpn",
    "surfshark",
    "pia-daemon",
    "cyberghost",
    "protonvpn",
    "windscribe",
    "hotspotshield",
];

/// Well-known VPN service names (systemd / launchd / Windows services).
pub const VPN_SERVICE_NAMES: &[&str] = &[
    "openvpn",
    "wg-quick@",
    "nordvpnd",
    "mullvad-daemon",
    "expressvpn",
];

/// Well-known Tor-related process names.
pub const TOR_PROCESS_NAMES: &[&str] = &[
    "tor",
    "tor-browser",
    "torbrowser",
    "obfs4proxy",
    "snowflake-client",
];

/// Network interface name prefixes commonly used by VPN software.
pub const VPN_INTERFACE_PREFIXES: &[&str] = &[
    "tun",
    "tap",
    "wg",
    "utun",
    "gpd",
    "ppp",
    "nordlynx",
    "proton",
    "mullvad",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_lists_non_empty() {
        assert!(!VPN_PROCESS_NAMES.is_empty());
        assert!(!VPN_SERVICE_NAMES.is_empty());
        assert!(!TOR_PROCESS_NAMES.is_empty());
        assert!(!VPN_INTERFACE_PREFIXES.is_empty());
    }

    #[test]
    fn no_duplicates_in_vpn_process_names() {
        let set: HashSet<&&str> = VPN_PROCESS_NAMES.iter().collect();
        assert_eq!(set.len(), VPN_PROCESS_NAMES.len());
    }

    #[test]
    fn no_duplicates_in_vpn_service_names() {
        let set: HashSet<&&str> = VPN_SERVICE_NAMES.iter().collect();
        assert_eq!(set.len(), VPN_SERVICE_NAMES.len());
    }

    #[test]
    fn no_duplicates_in_tor_process_names() {
        let set: HashSet<&&str> = TOR_PROCESS_NAMES.iter().collect();
        assert_eq!(set.len(), TOR_PROCESS_NAMES.len());
    }

    #[test]
    fn no_duplicates_in_vpn_interface_prefixes() {
        let set: HashSet<&&str> = VPN_INTERFACE_PREFIXES.iter().collect();
        assert_eq!(set.len(), VPN_INTERFACE_PREFIXES.len());
    }
}
