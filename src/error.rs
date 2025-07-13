use thiserror::Error;


#[derive(Error, Debug)]
pub enum StreamError {
    #[error("Invalid request")]
    InvalidRequest,

    #[error("Invalid url scheme")]
    InvalidScheme,

    #[error("No hostname")]
    NoHostname,

    #[error("Emtpy buff")]
    EmptyBuff
}