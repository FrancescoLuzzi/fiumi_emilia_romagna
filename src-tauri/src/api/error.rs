#[derive(thiserror::Error, Debug)]
pub enum StationsError {
    #[error("Couldn't get stations")]
    Decode(#[from] reqwest::Error),
    #[error("Couldn't parse url")]
    Parse(#[from] url::ParseError),
    #[error("Couldn't parse timeseries")]
    Timeseries(#[from] serde_json::Error),
}

impl serde::Serialize for StationsError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
