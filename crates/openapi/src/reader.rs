use std::io::Read;

use crate::{error::Error, types::Document};

pub fn load(uri: &str) -> Result<Document, Error> {
    let mut bytes = Vec::new();
    let res = ureq::get(uri).call().map_err(Error::Ureq)?;
    let (_, body) = res.into_parts();
    let _ = body.into_reader().read_to_end(&mut bytes);
    serde_json::from_slice::<Document>(&bytes).map_err(Error::SerdeJson)
}
