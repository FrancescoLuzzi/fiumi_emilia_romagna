#[derive(thiserror::Error, Debug)]
pub enum StationsError {
    #[error("Couldn't get stations")]
    Decode(#[from] reqwest::Error),
    #[error("Couldn't parse url")]
    Parse(#[from] url::ParseError),
    #[error("Couldn't parse timeseries")]
    Timeseries(#[from] serde_json::Error),
}
