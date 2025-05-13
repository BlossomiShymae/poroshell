use derive_more::{ Display, Error, From };
use irelia::requests::{ HyperError };

#[derive(Error, Debug, Display, From)]
pub enum Error {
    Io(std::io::Error),
    Fmt(std::fmt::Error),
    Json(serde_json::Error),
    IreliaHyper(irelia::error::Error<HyperError>),
}
