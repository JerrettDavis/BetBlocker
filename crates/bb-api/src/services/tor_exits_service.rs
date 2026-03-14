use sqlx::PgPool;

use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Fetch all Tor exit node IP addresses from the database.
///
/// Returns a `Vec<String>` so the route handler can serialize them without
/// depending on `std::net::IpAddr`.
pub async fn list_exit_nodes(db: &PgPool) -> Result<Vec<String>, ApiError> {
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT ip_address::text FROM tor_exit_nodes ORDER BY ip_address",
    )
    .fetch_all(db)
    .await?;

    Ok(rows)
}

/// Replace the entire Tor exit node table with a fresh list.
///
/// Called by the worker's `TorExitNodeRefreshJob` via direct DB access
/// (workers share the same schema, not the HTTP API).  Exposed here so tests
/// can call it directly and for potential future admin use.
pub async fn replace_exit_nodes(db: &PgPool, ips: &[String]) -> Result<usize, ApiError> {
    let mut tx = db.begin().await?;

    sqlx::query("DELETE FROM tor_exit_nodes")
        .execute(&mut *tx)
        .await?;

    let mut inserted = 0usize;
    for ip in ips {
        let rows = sqlx::query(
            "INSERT INTO tor_exit_nodes (ip_address) VALUES ($1::inet) ON CONFLICT (ip_address) DO NOTHING",
        )
        .bind(ip)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        inserted += rows as usize;
    }

    tx.commit().await?;
    Ok(inserted)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    /// Unit tests that don't hit a DB verify structural concerns only.
    #[test]
    fn module_accessible() {
        // Verifies that the public API compiles.
        let _ = std::any::type_name_of_val(&super::list_exit_nodes);
        let _ = std::any::type_name_of_val(&super::replace_exit_nodes);
    }
}
