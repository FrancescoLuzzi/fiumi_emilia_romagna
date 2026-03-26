mod error;

use chrono::{DateTime, DurationRound as _, Local, TimeDelta, TimeZone};

pub use crate::api::error::StationsError;
use crate::model::{Station, Stations, TimeSeries, TimeValue};

const STATIONS_URL: &str = "https://allertameteo.regione.emilia-romagna.it/o/api/allerta/get-sensor-values?variabile=254,0,0/1,-,-,-/B13215";
const TIMESERIES_URL: &str =
    "https://allertameteo.regione.emilia-romagna.it/o/api/allerta/get-time-series/";
pub const DELTA_15MIN: TimeDelta = TimeDelta::minutes(15);

#[derive(Clone, Debug)]
pub struct AlertClient {
    client: reqwest::Client,
}

impl Default for AlertClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AlertClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn stations_at<T>(&self, time: DateTime<T>) -> Result<Stations, StationsError>
    where
        T: TimeZone,
    {
        let mut call = reqwest::Url::parse(STATIONS_URL)?;
        call.query_pairs_mut()
            .encoding_override(Some(&|s| s.as_bytes().into()))
            .append_pair("time", &time.timestamp_millis().to_string());

        let mut stations: Vec<Station> = self.client.get(call).send().await?.json().await?;
        stations.sort_by(|a, b| b.cmp(a));
        Ok(Stations::new(stations))
    }

    pub async fn station_timeseries(&self, station_id: &str) -> Result<TimeSeries, StationsError> {
        let series = self
            .client
            .get(TIMESERIES_URL)
            .query(&[
                ("stazione", station_id),
                ("variabile", "254,0,0/1,-,-,-/B13215"),
            ])
            .send()
            .await?
            .json::<Vec<TimeValue>>()
            .await?;

        Ok(TimeSeries::new(series))
    }

    pub async fn latest_stations(&self) -> Result<Stations, StationsError> {
        let now = latest_station_time()?;
        self.stations_at(now).await
    }
}

pub fn latest_station_time() -> Result<DateTime<Local>, StationsError> {
    let adjusted = Local::now();
    adjusted
        .duration_trunc(DELTA_15MIN)
        .map_err(|err| StationsError::Unknown(err.to_string()))
}

pub fn clamp_station_time(date: DateTime<Local>) -> Result<DateTime<Local>, StationsError> {
    date.duration_trunc(DELTA_15MIN)
        .map_err(|err| StationsError::Unknown(err.to_string()))
}

pub async fn get_stations<T>(time: DateTime<T>) -> Result<Stations, StationsError>
where
    T: TimeZone,
{
    AlertClient::new().stations_at(time).await
}

pub async fn get_station_timeseries(station: &Station) -> Result<TimeSeries, StationsError> {
    AlertClient::new()
        .station_timeseries(station.idstazione())
        .await
}

pub async fn get_stations_now() -> Result<Stations, StationsError> {
    AlertClient::new().latest_stations().await
}
