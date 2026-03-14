use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::error::ApiError;
use crate::services::analytics_service;

// ---------------------------------------------------------------------------
// CSV Export
// ---------------------------------------------------------------------------

/// Generate a CSV report of daily block stats for a device within a time range.
///
/// Returns raw CSV bytes with headers:
/// date, event_type, event_count
pub async fn generate_csv_report(
    db: &PgPool,
    device_id: i64,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<u8>, ApiError> {
    let daily = analytics_service::get_daily_stats(db, device_id, from, to).await?;
    let trends = analytics_service::get_trends(db, device_id, &[]).await?;

    let mut wtr = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(vec![]);

    // Header row is written automatically by csv crate when we write a record
    // Write column headers
    wtr.write_record(["date", "event_type", "event_count"])
        .map_err(|e| ApiError::Internal {
            message: format!("CSV write error: {e}"),
        })?;

    for row in &daily {
        wtr.write_record([
            row.day.format("%Y-%m-%d").to_string(),
            row.event_type.clone(),
            row.event_count.to_string(),
        ])
        .map_err(|e| ApiError::Internal {
            message: format!("CSV write error: {e}"),
        })?;
    }

    // Append a blank separator then trend data
    if !trends.is_empty() {
        wtr.write_record(["", "", ""])
            .map_err(|e| ApiError::Internal {
                message: format!("CSV write error: {e}"),
            })?;
        wtr.write_record(["metric_name", "metric_value", "computed_at"])
            .map_err(|e| ApiError::Internal {
                message: format!("CSV write error: {e}"),
            })?;
        for t in &trends {
            wtr.write_record([
                t.metric_name.clone(),
                t.metric_value.to_string(),
                t.computed_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            ])
            .map_err(|e| ApiError::Internal {
                message: format!("CSV write error: {e}"),
            })?;
        }
    }

    let data = wtr.into_inner().map_err(|e| ApiError::Internal {
        message: format!("CSV flush error: {e}"),
    })?;

    Ok(data)
}

// ---------------------------------------------------------------------------
// PDF Export (text-based using printpdf)
// ---------------------------------------------------------------------------

/// Generate a PDF report for a device within a time range.
///
/// Produces a formatted single-page PDF with:
/// - Header / title block
/// - Summary statistics (total events, blocks, bypass attempts, tamper events)
/// - Daily counts table
/// - Trend highlights
pub async fn generate_pdf_report(
    db: &PgPool,
    device_id: i64,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<u8>, ApiError> {
    use printpdf::*;

    let summary = analytics_service::get_summary(db, device_id, from, to).await?;
    let daily = analytics_service::get_daily_stats(db, device_id, from, to).await?;
    let trends = analytics_service::get_trends(db, device_id, &[]).await?;

    let (doc, page1, layer1) = PdfDocument::new(
        "BetBlocker Analytics Report",
        Mm(210.0),
        Mm(297.0),
        "Layer 1",
    );

    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Load built-in font
    let font = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| ApiError::Internal {
            message: format!("PDF font error: {e}"),
        })?;
    let font_regular = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| ApiError::Internal {
            message: format!("PDF font error: {e}"),
        })?;

    // Title
    current_layer.use_text("BetBlocker Analytics Report", 20.0, Mm(20.0), Mm(277.0), &font);

    // Subtitle / date range
    let subtitle = format!(
        "Device {}  |  {} to {}",
        device_id,
        from.format("%Y-%m-%d"),
        to.format("%Y-%m-%d")
    );
    current_layer.use_text(&subtitle, 10.0, Mm(20.0), Mm(269.0), &font_regular);

    // Horizontal rule placeholder (move y down)
    let mut y = 258.0_f32;

    // Summary section
    current_layer.use_text("Summary", 14.0, Mm(20.0), Mm(y), &font);
    y -= 8.0;
    let summary_lines = [
        format!("Total Events:         {}", summary.total_events),
        format!("Total Blocks:         {}", summary.total_blocks),
        format!("Bypass Attempts:      {}", summary.total_bypass_attempts),
        format!("Tamper Events:        {}", summary.total_tamper_events),
        format!("Unique Event Types:   {}", summary.unique_event_types),
    ];
    for line in &summary_lines {
        current_layer.use_text(line, 10.0, Mm(20.0), Mm(y), &font_regular);
        y -= 6.0;
    }

    y -= 6.0;

    // Daily stats section
    current_layer.use_text("Daily Statistics", 14.0, Mm(20.0), Mm(y), &font);
    y -= 8.0;
    current_layer.use_text("Date            Event Type             Count", 9.0, Mm(20.0), Mm(y), &font);
    y -= 5.0;

    for row in daily.iter().take(30) {
        if y < 30.0 {
            break; // avoid overflow for large datasets (single page)
        }
        let line = format!(
            "{:<16}{:<23}{}",
            row.day.format("%Y-%m-%d"),
            row.event_type,
            row.event_count
        );
        current_layer.use_text(&line, 9.0, Mm(20.0), Mm(y), &font_regular);
        y -= 5.0;
    }

    y -= 6.0;

    // Trend highlights
    if !trends.is_empty() && y > 40.0 {
        current_layer.use_text("Trend Highlights", 14.0, Mm(20.0), Mm(y), &font);
        y -= 8.0;
        for trend in trends.iter().take(5) {
            if y < 30.0 {
                break;
            }
            let line = format!(
                "{}: {}  ({})",
                trend.metric_name,
                trend.metric_value,
                trend.computed_at.format("%Y-%m-%d")
            );
            current_layer.use_text(&line, 9.0, Mm(20.0), Mm(y), &font_regular);
            y -= 5.0;
        }
    }

    // Footer
    current_layer.use_text(
        &format!("Generated: {}", Utc::now().format("%Y-%m-%dT%H:%M:%SZ")),
        8.0,
        Mm(20.0),
        Mm(15.0),
        &font_regular,
    );

    let bytes = doc.save_to_bytes().map_err(|e| ApiError::Internal {
        message: format!("PDF save error: {e}"),
    })?;

    Ok(bytes)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    // CSV parsing test — uses an in-memory CSV and verifies structure without a DB.
    #[test]
    fn csv_output_parses_correctly() {
        // Build a minimal CSV manually the same way the service would.
        let mut wtr = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(vec![]);

        wtr.write_record(["date", "event_type", "event_count"]).unwrap();
        wtr.write_record(["2024-01-01", "block", "42"]).unwrap();
        wtr.write_record(["2024-01-02", "bypass_attempt", "3"]).unwrap();

        let data = wtr.into_inner().unwrap();
        let csv_str = String::from_utf8(data).unwrap();

        let mut rdr = csv::Reader::from_reader(csv_str.as_bytes());

        // Manually skip header and parse records
        let mut rdr2 = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(csv_str.as_bytes());

        let records: Vec<csv::StringRecord> = rdr2.records().map(|r| r.unwrap()).collect();
        assert_eq!(records.len(), 2);
        assert_eq!(&records[0][0], "2024-01-01");
        assert_eq!(&records[0][1], "block");
        assert_eq!(&records[0][2], "42");

        // Suppress unused warning
        let _ = rdr;
    }

    #[test]
    fn pdf_report_is_nonempty() {
        use printpdf::*;

        let (doc, page1, layer1) =
            PdfDocument::new("Test Report", Mm(210.0), Mm(297.0), "Layer 1");
        let layer = doc.get_page(page1).get_layer(layer1);
        let font = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();
        layer.use_text("Hello PDF", 12.0, Mm(20.0), Mm(277.0), &font);

        let bytes = doc.save_to_bytes().unwrap();
        assert!(!bytes.is_empty(), "PDF output should be non-empty");
        // PDF magic bytes
        assert!(bytes.starts_with(b"%PDF"), "Should start with PDF magic bytes");
    }
}
