use derive_more::{ Display, Error, From };

#[derive(Error, Debug, Display, From)]
pub enum Error {
    Io(std::io::Error),
    Fmt(std::fmt::Error),
}
