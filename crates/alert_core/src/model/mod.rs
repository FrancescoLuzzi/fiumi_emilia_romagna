use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::Value;
use serde_with::{VecSkipError, serde_as};

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
        if self.value.is_none() {
            return 0;
        }
        let value = self.value.unwrap();
        if value > self.soglia3 {
            return 0b1000;
        }
        if value > self.soglia2 {
            return 0b0100;
        }
        if value > self.soglia1 {
            return 0b0010;
        }
        0
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
#[derive(Deserialize, Serialize, Clone)]
pub struct Stations(#[serde_as(as = "VecSkipError<_>")] Vec<Station>);

impl Stations {
    pub fn new(stations: Vec<Station>) -> Self {
        Self(stations)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Station> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn into_vec(self) -> Vec<Station> {
        self.0
    }

    pub fn sort_by_alert_desc(&mut self) {
        self.0.sort_by(|a, b| b.cmp(a));
    }
}

impl AsRef<[Station]> for Stations {
    fn as_ref(&self) -> &[Station] {
        &self.0
    }
}

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

impl TimeValue {
    pub fn timestamp(&self) -> u64 {
        self.t
    }

    pub fn value(&self) -> Option<f64> {
        self.v
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TimeSeries(Vec<TimeValue>);

impl TimeSeries {
    pub fn new(data: Vec<TimeValue>) -> Self {
        Self(data)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, TimeValue> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Converts the time series into chart points using Unix timestamps in milliseconds on the x axis.
    ///
    /// The upstream API returns roughly 250 readings spaced 15 minutes apart, with the latest
    /// reading aligned with the current timestamp. Consumers should therefore treat the x axis as
    /// real time and render at least one label per day for readability.
    pub fn as_dataset(self) -> Vec<(f64, f64)> {
        self.0
            .into_iter()
            .map(|tv| (tv.timestamp() as f64, tv.value().unwrap_or(0.0)))
            .collect()
    }
}
