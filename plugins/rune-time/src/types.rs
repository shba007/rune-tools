use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurrentTimeResponse {
    pub timezone: String,
    pub datetime: String,
    pub utc_datetime: String,
    pub utc_offset: String,
    pub timestamp_epoch_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimezoneDetails {
    pub timezone: String,
    pub datetime: String,
    pub utc_offset: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConvertTimeResponse {
    pub source: TimezoneDetails,
    pub target: TimezoneDetails,
    pub time_difference_hours: f64,
}
