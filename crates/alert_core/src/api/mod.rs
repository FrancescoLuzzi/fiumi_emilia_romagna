mod error;
use chrono::{DurationRound as _, TimeDelta};

pub use crate::api::error::StationsError;
use crate::model::{Station, Stations, TimeSeries, TimeValue};

// the region apis are a little weird in the TZ department
const DELTA_2H_20MIN: TimeDelta = TimeDelta::minutes(140);
const DELTA_15MIN: TimeDelta = TimeDelta::minutes(15);

pub fn get_stations<T>(time: chrono::DateTime<T>) -> Result<Stations, error::StationsError>
where
    T: chrono::TimeZone,
{
    let now = time.timestamp_millis();
    let mut call = reqwest::Url::parse(
        "https://allertameteo.regione.emilia-romagna.it/o/api/allerta/get-sensor-values?variabile=254,0,0/1,-,-,-/B13215",
    )?;
    call.query_pairs_mut()
        .encoding_override(Some(&|s| s.as_bytes().into()))
        .append_pair("time", &now.to_string());
    let mut stations: Vec<Station> = reqwest::blocking::get(call)?.json::<Vec<_>>()?;
    stations.sort_by(|a, b| b.cmp(a));
    Ok(Stations(stations))
}

pub fn get_station_timeseries(station: &Station) -> Result<TimeSeries, error::StationsError> {
    let time_series = reqwest::blocking::get(format!("https://allertameteo.regione.emilia-romagna.it/o/api/allerta/get-time-series/?stazione={}&variabile=254,0,0/1,-,-,-/B13215",station.idstazione()))?.json::<Vec<TimeValue>>()?;
    Ok(TimeSeries::new(time_series))
}

pub fn get_stations_now() -> Result<Stations, StationsError> {
    let mut now = chrono::Local::now();
    now -= DELTA_2H_20MIN;
    now = now
        .duration_trunc(DELTA_15MIN)
        .map_err(|err| StationsError::Unknown(err.to_string()))?;
    get_stations(now)
}
