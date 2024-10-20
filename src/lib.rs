pub mod event_handler_trait;
pub mod graph;
pub mod table;
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use serde_with::{serde_as, VecSkipError};

#[derive(Debug, Deserialize, Serialize, Clone)]
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
    fn score(&self) -> u8 {
        let value = self.value;
        if value.is_none() {
            return 0;
        }
        let mut outval: u8 = 0;
        let value = value.unwrap();
        if value > self.soglia1 {
            outval |= 0b0010;
        }
        if value > self.soglia2 {
            outval |= 0b0100;
        }
        if value > self.soglia3 {
            outval |= 0b1000;
        }
        outval
    }
}

impl PartialEq for Station {
    fn eq(&self, other: &Self) -> bool {
        self.idstazione == other.idstazione
    }
}

impl Eq for Station {}

impl PartialOrd for Station {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Station {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let mut out = self.score().cmp(&other.score());
        if matches!(out, std::cmp::Ordering::Equal) {
            out = self
                .value
                .partial_cmp(&other.value)
                .unwrap_or(std::cmp::Ordering::Less);
        }
        out
    }
}

#[serde_as]
#[derive(Deserialize, Serialize)]
pub struct Stations(#[serde_as(as = "VecSkipError<_>")] pub Vec<Station>);

fn de_timestamp<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    Ok(match Value::deserialize(deserializer)? {
        Value::Number(num) => num.as_u64().ok_or(de::Error::custom("Invalid number"))?,
        Value::String(s) => s.parse::<u64>().map_err(de::Error::custom)?,
        _ => return Err(de::Error::custom("wrong type")),
    })
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeValue {
    #[serde(deserialize_with = "de_timestamp")]
    t: u64,
    v: Option<f64>,
}

pub struct TimeSeries(Vec<TimeValue>);

impl TimeSeries {
    pub fn new(data: Vec<TimeValue>) -> Self {
        Self(data)
    }
    pub fn as_dataset(self) -> Vec<(f64, f64)> {
        let _t0 = self.0.first().unwrap().t;
        self.0
            .into_iter()
            .enumerate()
            .map(|(i, tv)| (i as f64, tv.v.unwrap_or(0.)))
            .collect()
    }
}
