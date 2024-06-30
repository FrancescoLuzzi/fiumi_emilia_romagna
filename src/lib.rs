pub mod event_handler_trait;
pub mod graph;
pub mod table;
use chrono::{serde::ts_milliseconds, DateTime, Utc};

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct Station {
    pub idstazione: String,
    pub ordinamento: usize,
    pub nomestaz: String,
    pub lon: String,
    pub lat: String,
    pub value: Option<f32>,
    pub soglia1: f32,
    pub soglia2: f32,
    pub soglia3: f32,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TimeValue {
    #[serde(with = "ts_milliseconds")]
    t: DateTime<Utc>,
    v: f32,
}
