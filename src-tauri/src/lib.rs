use api::StationsError;
use chrono::Utc;
use model::{Station, Stations, TimeSeries};

pub mod api;
pub mod fiumi_lib;
pub mod model;

#[tauri::command]
fn get_stations_now() -> Result<Stations, StationsError> {
    api::get_stations_now()
}

#[tauri::command]
fn get_stations(date: chrono::DateTime<Utc>) -> Result<Stations, StationsError> {
    api::get_stations(date)
}

#[tauri::command]
fn get_time_series(station: Station) -> Result<TimeSeries, StationsError> {
    api::get_station_timeseries(&station)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_stations_now,
            get_time_series,
            get_stations
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
