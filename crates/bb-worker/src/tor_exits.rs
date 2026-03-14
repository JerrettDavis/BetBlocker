use crate::scheduler::AppContext;

/// URL of the Tor Project's bulk exit list.
const TOR_BULK_EXIT_LIST_URL: &str =
    "https://check.torproject.org/torbulkexitlist";

// ---------------------------------------------------------------------------
// TorExitNodeRefreshJob
// ---------------------------------------------------------------------------

/// Fetches the Tor Project's bulk exit node list and stores it in the
/// `tor_exit_nodes` database table.
///
/// Scheduled every 6 hours.
pub struct TorExitNodeRefreshJob;

impl TorExitNodeRefreshJob {
    /// Run one refresh cycle.
    ///
    /// 1. Fetch the plain-text IP list from torproject.org.
    /// 2. Parse valid IP addresses (skip comments and blanks).
    /// 3. Replace the `tor_exit_nodes` table contents atomically.
    pub async fn run(ctx: &AppContext) -> anyhow::Result<()> {
        tracing::info!("tor_exit_refresh: fetching exit node list");

        let body = ctx
            .http
            .get(TOR_BULK_EXIT_LIST_URL)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let ips = parse_ip_list(&body);
        tracing::info!(count = ips.len(), "tor_exit_refresh: parsed IPs");

        replace_exit_nodes(&ctx.db, &ips).await?;
        tracing::info!(count = ips.len(), "tor_exit_refresh: database updated");

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a newline-delimited list of IPs, skipping comments and blanks.
pub fn parse_ip_list(data: &str) -> Vec<String> {
    data.lines()
        .filter_map(|line| {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                return None;
            }
            // Validate by attempting to parse as IpAddr.
            if t.parse::<std::net::IpAddr>().is_ok() {
                Some(t.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Replace all rows in `tor_exit_nodes` within a single transaction.
async fn replace_exit_nodes(db: &sqlx::PgPool, ips: &[String]) -> anyhow::Result<()> {
    let mut tx = db.begin().await?;

    sqlx::query("DELETE FROM tor_exit_nodes")
        .execute(&mut *tx)
        .await?;

    for ip in ips {
        sqlx::query(
            "INSERT INTO tor_exit_nodes (ip_address) VALUES ($1::inet) ON CONFLICT (ip_address) DO NOTHING",
        )
        .bind(ip)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_list() {
        let data = "1.2.3.4\n5.6.7.8\n";
        let ips = parse_ip_list(data);
        assert_eq!(ips.len(), 2);
        assert!(ips.contains(&"1.2.3.4".to_string()));
        assert!(ips.contains(&"5.6.7.8".to_string()));
    }

    #[test]
    fn parse_skips_comments_and_blanks() {
        let data = "# Tor exit nodes\n\n1.2.3.4\n\n# end\n5.6.7.8\n";
        let ips = parse_ip_list(data);
        assert_eq!(ips.len(), 2);
    }

    #[test]
    fn parse_skips_invalid_ips() {
        let data = "1.2.3.4\nnot_an_ip\n999.0.0.1\n::1\n";
        let ips = parse_ip_list(data);
        // Only 1.2.3.4 and ::1 are valid
        assert_eq!(ips.len(), 2);
        assert!(ips.contains(&"1.2.3.4".to_string()));
        assert!(ips.contains(&"::1".to_string()));
    }

    #[test]
    fn parse_empty_string_returns_empty() {
        assert!(parse_ip_list("").is_empty());
    }

    #[test]
    fn parse_only_comments_returns_empty() {
        let data = "# line1\n# line2\n";
        assert!(parse_ip_list(data).is_empty());
    }

    #[test]
    fn parse_ipv6_addresses() {
        let data = "2001:db8::1\n::1\nfe80::1\n";
        let ips = parse_ip_list(data);
        assert_eq!(ips.len(), 3);
    }

    /// Verify the constant URL is well-formed.
    #[test]
    fn tor_exit_list_url_is_https() {
        assert!(TOR_BULK_EXIT_LIST_URL.starts_with("https://"));
    }
}
