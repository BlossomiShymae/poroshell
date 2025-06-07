use derive_more::{ Display, Error, From };

#[derive(Debug, Display, Error)]
pub enum PoroError<E: std::error::Error> {
    #[display("client error: {_0}")] Client(E),
    #[display("parsing error: {_0}")] Parse(ParseError),
}

/// Blanket implementation for all errors that implement `Into<ParseError>`.
impl<T: Into<ParseError>, E: std::error::Error> From<T> for PoroError<E> {
    fn from(impl_into_parse_error: T) -> Self {
        PoroError::Parse(impl_into_parse_error.into())
    }
}

#[derive(Debug, Display, Error, From)]
pub enum ParseError {
    Io(std::io::Error),
    Fmt(std::fmt::Error),
    Json(serde_json::Error),
    PatchSyntax(SyntaxError),
    CannotParseEmptyStringIntoType,
    ConsoleEndpointResponseShouldBeObject,
    EndpointPathCannotBeNone,
    FormatIsNotAnInteger,
    FormatIsNotANumber,
    InvalidData,
    PrivateApiTypeNotSupported,
    UnknownHttpMethod,
    ObjectTypesShouldBeParsed,
    VectorTypesShouldBeParsed,
}

#[derive(Debug, Display, Error, From)]
pub enum SyntaxError {
    #[display("Wildcards are not valid members of a union")]
    WildcardInUnion,
}
