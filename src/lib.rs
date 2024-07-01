pub mod event_handler_trait;
pub mod graph;
pub mod table;
use chrono::{serde::ts_milliseconds, DateTime, Utc};

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct Station {
    idstazione: String,
    ordinamento: usize,
    nomestaz: String,
    lon: String,
    lat: String,
    value: Option<f32>,
    soglia1: f32,
    soglia2: f32,
    soglia3: f32,
}

impl Station {
    fn ref_array(&self) -> [String; 5] {
        [
            self.nomestaz.clone(),
            self.value.unwrap_or(0.0).to_string(),
            self.soglia1.to_string(),
            self.soglia2.to_string(),
            self.soglia3.to_string(),
        ]
    }

    fn nomestaz(&self) -> &str {
        &self.nomestaz
    }

    fn value(&self) -> Option<&f32> {
        self.value.as_ref()
    }
    fn soglia1(&self) -> &f32 {
        &self.soglia1
    }
    fn soglia2(&self) -> &f32 {
        &self.soglia2
    }
    fn soglia3(&self) -> &f32 {
        &self.soglia3
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TimeValue {
    #[serde(with = "ts_milliseconds")]
    t: DateTime<Utc>,
    v: f32,
}
