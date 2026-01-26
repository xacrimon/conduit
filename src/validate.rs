use thiserror::Error;

pub trait Validate {
    fn validate(&self) -> Result<(), ValidationError>;
}

#[derive(Error, Debug)]
#[error("validation error")]
pub struct ValidationError;
