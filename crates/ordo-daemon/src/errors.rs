use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonErrorCode {
    Internal,
    Forbidden,
    InvalidRequest,
}

impl DaemonErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Internal => "internal_error",
            Self::Forbidden => "forbidden",
            Self::InvalidRequest => "invalid_request",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub code: String,
    pub error: String,
}

impl ErrorResponse {
    pub fn new(code: DaemonErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.as_str().to_string(),
            error: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_stable_public_strings() {
        assert_eq!(DaemonErrorCode::Internal.as_str(), "internal_error");
        assert_eq!(DaemonErrorCode::Forbidden.as_str(), "forbidden");
        assert_eq!(DaemonErrorCode::InvalidRequest.as_str(), "invalid_request");
    }
}
