use std::fmt;

pub type OrichResult<T> = Result<T, OrichError>;

#[derive(Debug, Clone, PartialEq)]
pub struct OrichError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl OrichError {
    pub fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            line,
            column,
        }
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::new(message, 0, 0)
    }
}

impl fmt::Display for OrichError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{} at {}:{}", self.message, self.line, self.column)
        }
    }
}

impl std::error::Error for OrichError {}
