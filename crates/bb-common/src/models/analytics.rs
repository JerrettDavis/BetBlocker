use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Hourly aggregated block statistics from the continuous aggregate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyBlockStat {
    pub bucket: DateTime<Utc>,
    pub device_id: i64,
    pub event_type: String,
    pub event_count: i64,
}

/// Daily aggregated block statistics rolled up from hourly stats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyBlockStat {
    pub day: DateTime<Utc>,
    pub device_id: i64,
    pub event_type: String,
    pub event_count: i64,
}

/// Pre-computed analytics trend for a device metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsTrend {
    pub id: i64,
    pub device_id: i64,
    pub metric_name: String,
    pub metric_value: serde_json::Value,
    pub computed_at: DateTime<Utc>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hourly_block_stat_roundtrips_json() {
        let stat = HourlyBlockStat {
            bucket: Utc::now(),
            device_id: 42,
            event_type: "block".to_string(),
            event_count: 150,
        };
        let json = serde_json::to_string(&stat).unwrap();
        let back: HourlyBlockStat = serde_json::from_str(&json).unwrap();
        assert_eq!(stat.device_id, back.device_id);
        assert_eq!(stat.event_type, back.event_type);
        assert_eq!(stat.event_count, back.event_count);
    }

    #[test]
    fn daily_block_stat_roundtrips_json() {
        let stat = DailyBlockStat {
            day: Utc::now(),
            device_id: 7,
            event_type: "bypass_attempt".to_string(),
            event_count: 30,
        };
        let json = serde_json::to_string(&stat).unwrap();
        let back: DailyBlockStat = serde_json::from_str(&json).unwrap();
        assert_eq!(stat.device_id, back.device_id);
        assert_eq!(stat.event_count, back.event_count);
    }

    #[test]
    fn analytics_trend_roundtrips_json() {
        let trend = AnalyticsTrend {
            id: 1,
            device_id: 42,
            metric_name: "block_rate_change".to_string(),
            metric_value: serde_json::json!({"delta": 0.15, "direction": "up"}),
            computed_at: Utc::now(),
            period_start: Utc::now(),
            period_end: Utc::now(),
        };
        let json = serde_json::to_string(&trend).unwrap();
        let back: AnalyticsTrend = serde_json::from_str(&json).unwrap();
        assert_eq!(trend.id, back.id);
        assert_eq!(trend.metric_name, back.metric_name);
        assert_eq!(trend.metric_value, back.metric_value);
    }
}
