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

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::Internal {
            message: format!("JSON error: {}", e),
        }
    }
}
