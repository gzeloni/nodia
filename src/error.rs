use std::fmt;

pub type DobraResult<T> = Result<T, DobraError>;

#[derive(Debug, Clone, PartialEq)]
pub struct DobraError {
    pub code: String,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub file: Option<String>,
}

impl DobraError {
    pub fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            code: "E1000".to_string(),
            message: message.into(),
            line,
            column,
            file: None,
        }
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self {
            code: "E2000".to_string(),
            message: message.into(),
            line: 0,
            column: 0,
            file: None,
        }
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self {
            code: "E3000".to_string(),
            message: message.into(),
            line: 0,
            column: 0,
            file: None,
        }
    }

    pub fn semantic(message: impl Into<String>) -> Self {
        Self::semantic_at(message, 0, 0)
    }

    pub fn semantic_at(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            code: "E4000".to_string(),
            message: message.into(),
            line,
            column,
            file: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = code.into();
        self
    }

    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn with_file_if_missing(mut self, file: impl Into<String>) -> Self {
        if self.file.is_none() {
            self.file = Some(file.into());
        }
        self
    }

    pub fn render(&self) -> String {
        let location = match (&self.file, self.line, self.column) {
            (Some(file), line, column) if line > 0 => format!("\n  at {file}:{line}:{column}"),
            (None, line, column) if line > 0 => format!("\n  at {line}:{column}"),
            (Some(file), _, _) => format!("\n  at {file}"),
            _ => String::new(),
        };
        format!("error[{}]: {}{}", self.code, self.message, location)
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\"code\":\"{}\",\"message\":\"{}\",\"file\":{},\"line\":{},\"column\":{}}}",
            json_escape(&self.code),
            json_escape(&self.message),
            self.file
                .as_ref()
                .map(|file| format!("\"{}\"", json_escape(file)))
                .unwrap_or_else(|| "null".to_string()),
            self.line,
            self.column
        )
    }
}

impl fmt::Display for DobraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

impl std::error::Error for DobraError {}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}
