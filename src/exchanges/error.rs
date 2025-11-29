use std::fmt;

/// Error wrapper for exchange adapters
/// This allows us to use Box<dyn Error> as a concrete error type
#[derive(Debug)]
pub struct ExchangeError {
    inner: Box<dyn std::error::Error + Send + Sync>,
}

impl ExchangeError {
    pub fn new<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        Self {
            inner: Box::new(error),
        }
    }

    pub fn from_box(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self { inner: error }
    }
}

impl fmt::Display for ExchangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl std::error::Error for ExchangeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source()
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for ExchangeError {
    fn from(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self { inner: error }
    }
}

/// Boxed error type that implements Error trait
/// This is a workaround for using Box<dyn Error> as an associated type
#[derive(Debug)]
pub struct BoxedError(Box<dyn std::error::Error + Send + Sync>);

impl BoxedError {
    pub fn new<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        Self(Box::new(error))
    }

    pub fn from_box(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self(error)
    }
}

impl fmt::Display for BoxedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BoxedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for BoxedError {
    fn from(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self(error)
    }
}
