use derive_more::{Display, Error, From};

#[derive(Error, Debug, Display, From)]
pub enum Error {
    Ureq(ureq::Error),
    SerdeJson(serde_json::error::Error),
}
