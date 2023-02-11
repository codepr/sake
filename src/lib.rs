pub mod mqtt;

pub type AsyncResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub type SerdeResult<T> = std::result::Result<T, Box<bincode::ErrorKind>>;
