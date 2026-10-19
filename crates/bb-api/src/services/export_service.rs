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
#[allow(clippy::items_after_statements)]
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

    // Minimal text-layer helper that accumulates printpdf page operations.
    struct TextLayer {
        ops: Vec<Op>,
    }

    impl TextLayer {
        fn use_text(
            &mut self,
            text: impl AsRef<str>,
            size: f32,
            x: Mm,
            y: Mm,
            font: &PdfFontHandle,
        ) {
            self.ops.extend([
                Op::StartTextSection,
                Op::SetTextCursor {
                    pos: Point::new(x, y),
                },
                Op::SetFont {
                    font: font.clone(),
                    size: Pt(size),
                },
                Op::ShowText {
                    items: vec![TextItem::Text(text.as_ref().to_string())],
                },
                Op::EndTextSection,
            ]);
        }
    }

    let mut current_layer = TextLayer { ops: Vec::new() };
    let font = PdfFontHandle::Builtin(BuiltinFont::HelveticaBold);
    let font_regular = PdfFontHandle::Builtin(BuiltinFont::Helvetica);

    // Title
    current_layer.use_text(
        "BetBlocker Analytics Report",
        20.0,
        Mm(20.0),
        Mm(277.0),
        &font,
    );

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
    current_layer.use_text(
        "Date            Event Type             Count",
        9.0,
        Mm(20.0),
        Mm(y),
        &font,
    );
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

    let mut doc = PdfDocument::new("BetBlocker Analytics Report");
    let page = PdfPage::new(Mm(210.0), Mm(297.0), current_layer.ops);
    let mut warnings = Vec::new();
    let bytes = doc
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut warnings);

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

        wtr.write_record(["date", "event_type", "event_count"])
            .unwrap();
        wtr.write_record(["2024-01-01", "block", "42"]).unwrap();
        wtr.write_record(["2024-01-02", "bypass_attempt", "3"])
            .unwrap();

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

        let mut doc = PdfDocument::new("Test Report");
        let ops = vec![
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point::new(Mm(20.0), Mm(277.0)),
            },
            Op::SetFont {
                font: PdfFontHandle::Builtin(BuiltinFont::Helvetica),
                size: Pt(12.0),
            },
            Op::ShowText {
                items: vec![TextItem::Text("Hello PDF".to_string())],
            },
            Op::EndTextSection,
        ];
        let page = PdfPage::new(Mm(210.0), Mm(297.0), ops);
        let mut warnings = Vec::new();
        let bytes = doc
            .with_pages(vec![page])
            .save(&PdfSaveOptions::default(), &mut warnings);
        assert!(!bytes.is_empty(), "PDF output should be non-empty");
        // PDF magic bytes
        assert!(
            bytes.starts_with(b"%PDF"),
            "Should start with PDF magic bytes"
        );
    }
}
