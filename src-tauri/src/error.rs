use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{message}")]
    Coded {
        code: &'static str,
        message: String,
        details: Option<String>,
    },
}

impl AppError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self::Coded {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(
        code: &'static str,
        message: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self::Coded {
            code,
            message: message.into(),
            details: Some(details.into()),
        }
    }

    pub fn code(&self) -> &'static str {
        let Self::Coded { code, .. } = self;
        code
    }
}

#[derive(Serialize)]
struct ErrorDto {
    code: String,
    message: String,
    details: Option<String>,
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let Self::Coded {
            code,
            message,
            details,
        } = self;
        ErrorDto {
            code: (*code).into(),
            message: message.clone(),
            details: details.clone(),
        }
        .serialize(serializer)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(value: rusqlite::Error) -> Self {
        Self::new("internal", format!("database error: {value}"))
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::new("internal", format!("io error: {value}"))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        Self::new("invalid_config", format!("json error: {value}"))
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        if value.is_timeout() {
            return Self::new("timeout", "request timed out");
        }
        if value.is_connect() {
            return Self::new("network", value.to_string());
        }
        if let Some(status) = value.status() {
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Self::new("unauthorized", "unauthorized");
            }
            if status.as_u16() == 404 {
                return Self::new("not_found", "endpoint not found");
            }
        }
        Self::new("network", value.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
