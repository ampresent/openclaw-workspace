use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// All agent errors
#[derive(Debug)]
pub enum AppError {
    /// Command execution failed
    CommandFailed {
        command: String,
        message: String,
    },
    /// File I/O error
    IoError {
        path: String,
        message: String,
    },
    /// Invalid request parameters
    Validation {
        field: String,
        message: String,
    },
    /// Resource not found
    NotFound {
        resource: String,
    },
    /// Auth failed
    Unauthorized,
    /// Something unexpected
    Internal {
        message: String,
    },
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommandFailed { command, message } => {
                write!(f, "命令 {command} 执行失败: {message}")
            }
            Self::IoError { path, message } => {
                write!(f, "文件操作失败 ({path}): {message}")
            }
            Self::Validation { field, message } => {
                write!(f, "参数错误 ({field}): {message}")
            }
            Self::NotFound { resource } => {
                write!(f, "未找到: {resource}")
            }
            Self::Unauthorized => write!(f, "认证失败: 请检查 API token"),
            Self::Internal { message } => {
                write!(f, "内部错误: {message}")
            }
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            Self::CommandFailed { .. } => (StatusCode::INTERNAL_SERVER_ERROR, "COMMAND_FAILED"),
            Self::IoError { .. } => (StatusCode::INTERNAL_SERVER_ERROR, "IO_ERROR"),
            Self::Validation { .. } => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR"),
            Self::NotFound { .. } => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            Self::Internal { .. } => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };

        let body = json!({
            "error": {
                "code": code,
                "message": self.to_string(),
            }
        });

        (status, Json(body)).into_response()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError {
            path: "(unknown)".into(),
            message: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_failed_display() {
        let err = AppError::CommandFailed {
            command: "nixos-rebuild".into(),
            message: "build failed".into(),
        };
        let display = format!("{err}");
        assert!(display.contains("nixos-rebuild"));
        assert!(display.contains("build failed"));
    }

    #[test]
    fn test_io_error_display() {
        let err = AppError::IoError {
            path: "/etc/nixos".into(),
            message: "permission denied".into(),
        };
        let display = format!("{err}");
        assert!(display.contains("/etc/nixos"));
        assert!(display.contains("permission denied"));
    }

    #[test]
    fn test_validation_display() {
        let err = AppError::Validation {
            field: "config".into(),
            message: "must not be empty".into(),
        };
        let display = format!("{err}");
        assert!(display.contains("config"));
        assert!(display.contains("must not be empty"));
    }

    #[test]
    fn test_not_found_display() {
        let err = AppError::NotFound {
            resource: "environment base".into(),
        };
        let display = format!("{err}");
        assert!(display.contains("environment base"));
    }

    #[test]
    fn test_unauthorized_display() {
        let err = AppError::Unauthorized;
        let display = format!("{err}");
        assert!(display.contains("认证失败"));
    }

    #[test]
    fn test_internal_display() {
        let err = AppError::Internal {
            message: "something broke".into(),
        };
        let display = format!("{err}");
        assert!(display.contains("something broke"));
    }

    #[test]
    fn test_command_failed_into_response() {
        let err = AppError::CommandFailed {
            command: "test".into(),
            message: "fail".into(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_validation_into_response() {
        let err = AppError::Validation {
            field: "name".into(),
            message: "required".into(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_not_found_into_response() {
        let err = AppError::NotFound {
            resource: "thing".into(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_unauthorized_into_response() {
        let err = AppError::Unauthorized;
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_io_error_into_response() {
        let err = AppError::IoError {
            path: "/tmp".into(),
            message: "err".into(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_internal_into_response() {
        let err = AppError::Internal {
            message: "oops".into(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let app_err: AppError = io_err.into();
        match app_err {
            AppError::IoError { message, .. } => {
                assert!(message.contains("file not found"));
            }
            _ => panic!("Expected IoError"),
        }
    }

    #[test]
    fn test_error_codes_in_response() {
        // Verify error codes are correct in JSON response
        let err = AppError::Validation {
            field: "x".into(),
            message: "y".into(),
        };
        let response = err.into_response();
        // The response should be JSON with error.code = "VALIDATION_ERROR"
        // We just check the status code since body extraction is async
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
