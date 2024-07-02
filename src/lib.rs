pub mod event_handler_trait;
pub mod graph;
pub mod table;
use std::f64;

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
    pub fn ref_array(&self) -> [String; 5] {
        [
            self.nomestaz.clone(),
            self.value.unwrap_or(0.0).to_string(),
            self.soglia1.to_string(),
            self.soglia2.to_string(),
            self.soglia3.to_string(),
        ]
    }
    pub fn idstazione(&self) -> &str {
        &self.idstazione
    }

    pub fn nomestaz(&self) -> &str {
        &self.nomestaz
    }

    pub fn value(&self) -> Option<&f32> {
        self.value.as_ref()
    }
    pub fn soglia1(&self) -> &f32 {
        &self.soglia1
    }
    pub fn soglia2(&self) -> &f32 {
        &self.soglia2
    }
    pub fn soglia3(&self) -> &f32 {
        &self.soglia3
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct TimeValue {
    t: u64,
    v: f64,
}

pub struct TimeSeries(Vec<TimeValue>);

impl TimeSeries {
    pub fn new(data: Vec<TimeValue>) -> Self {
        Self(data)
    }
    pub fn as_dataset(self) -> Vec<(f64, f64)> {
        let t0 = self.0.first().unwrap().t;
        self.0
            .into_iter()
            .map(|tv| (f64::from((tv.t - t0) as u32), tv.v))
            .collect()
    }
}
