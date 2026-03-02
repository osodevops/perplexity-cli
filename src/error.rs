use thiserror::Error;

#[derive(Debug, Error)]
pub enum PplxError {
    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("Rate limited. Retry after {retry_after_secs:?}s")]
    RateLimited { retry_after_secs: Option<u64> },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Research error: {0}")]
    Research(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl PplxError {
    /// Machine-readable error code for programmatic consumption.
    pub fn error_code(&self) -> &'static str {
        match self {
            PplxError::Auth(_) => "auth_failed",
            PplxError::Api { status, .. } => match *status {
                404 => "not_found",
                s if s >= 500 => "server_error",
                _ => "api_error",
            },
            PplxError::RateLimited { .. } => "rate_limited",
            PplxError::Config(_) => "config_error",
            PplxError::Stream(_) => "stream_error",
            PplxError::Validation(_) => "validation_error",
            PplxError::Research(_) => "research_error",
            PplxError::Http(_) => "http_error",
            PplxError::Io(_) => "io_error",
            PplxError::Json(_) => "json_error",
        }
    }

    /// Semantic process exit code.
    pub fn exit_code(&self) -> i32 {
        match self {
            PplxError::Validation(_) | PplxError::Config(_) => 2,
            PplxError::Auth(_) => 3,
            PplxError::RateLimited { .. } => 4,
            PplxError::Stream(_) | PplxError::Research(_) => 5,
            PplxError::Api { status, .. } if *status == 404 => 6,
            PplxError::Api { status, .. } if *status >= 500 => 5,
            PplxError::Api { .. } => 1,
            PplxError::Http(_) => 7,
            PplxError::Io(_) | PplxError::Json(_) => 1,
        }
    }

    /// Actionable suggestion for resolving this error.
    pub fn suggestion(&self) -> &'static str {
        match self {
            PplxError::Auth(_) => "Set PERPLEXITY_API_KEY or run `pplx config init`",
            PplxError::Api { status, .. } if *status == 404 => {
                "Check the model name or API endpoint"
            }
            PplxError::Api { status, .. } if *status >= 500 => {
                "Perplexity API server error — retry in a few seconds"
            }
            PplxError::Api { .. } => "Check the request parameters and try again",
            PplxError::RateLimited { .. } => "Wait and retry, or reduce request frequency",
            PplxError::Config(_) => "Check your config file or run `pplx config init`",
            PplxError::Stream(_) => "Try with --no-stream, or retry",
            PplxError::Validation(_) => {
                "Check the input values (see `pplx --help` for valid ranges)"
            }
            PplxError::Research(_) => "Check research job ID or retry submission",
            PplxError::Http(_) => "Check your network connection and try again",
            PplxError::Io(_) => "Check file permissions and disk space",
            PplxError::Json(_) => "Check the JSON input or report a bug",
        }
    }

    /// Structured JSON error for machine consumption.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "error": {
                "code": self.error_code(),
                "message": self.to_string(),
                "suggestion": self.suggestion(),
                "exit_code": self.exit_code(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let cases: Vec<(PplxError, &str)> = vec![
            (PplxError::Auth("bad key".into()), "auth_failed"),
            (
                PplxError::Api {
                    status: 404,
                    message: "not found".into(),
                },
                "not_found",
            ),
            (
                PplxError::Api {
                    status: 500,
                    message: "server".into(),
                },
                "server_error",
            ),
            (
                PplxError::Api {
                    status: 400,
                    message: "bad".into(),
                },
                "api_error",
            ),
            (
                PplxError::RateLimited {
                    retry_after_secs: Some(30),
                },
                "rate_limited",
            ),
            (PplxError::Config("bad".into()), "config_error"),
            (PplxError::Stream("err".into()), "stream_error"),
            (PplxError::Validation("err".into()), "validation_error"),
            (PplxError::Research("err".into()), "research_error"),
        ];
        for (err, expected_code) in cases {
            assert_eq!(err.error_code(), expected_code, "for {err:?}");
        }
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(PplxError::Auth("x".into()).exit_code(), 3);
        assert_eq!(PplxError::Validation("x".into()).exit_code(), 2);
        assert_eq!(PplxError::Config("x".into()).exit_code(), 2);
        assert_eq!(
            PplxError::RateLimited {
                retry_after_secs: None
            }
            .exit_code(),
            4
        );
        assert_eq!(PplxError::Stream("x".into()).exit_code(), 5);
        assert_eq!(PplxError::Research("x".into()).exit_code(), 5);
        assert_eq!(
            PplxError::Api {
                status: 404,
                message: "x".into()
            }
            .exit_code(),
            6
        );
        assert_eq!(
            PplxError::Api {
                status: 500,
                message: "x".into()
            }
            .exit_code(),
            5
        );
    }

    #[test]
    fn test_to_json_structure() {
        let err = PplxError::Auth("Invalid API key".into());
        let json = err.to_json();
        let obj = json["error"].as_object().unwrap();
        assert_eq!(obj["code"], "auth_failed");
        assert!(obj["message"].as_str().unwrap().contains("Invalid API key"));
        assert!(!obj["suggestion"].as_str().unwrap().is_empty());
        assert_eq!(obj["exit_code"], 3);
    }
}
