use axum::Json;
use serde::{Deserialize, Serialize};
use super::*;

#[derive(Deserialize)]
pub struct LogsQuery {
    pub host: Option<String>,
    pub unit: String,
    pub lines: Option<usize>,
}

#[derive(Serialize)]
pub struct LogsResponse {
    pub unit: String,
    pub lines: usize,
    pub logs: Vec<String>,
}

pub async fn handle(
    State(state): AppStateRef,
    Query(query): Query<LogsQuery>,
) -> Result<Json<LogsResponse>, String> {
    let lines = query.lines.unwrap_or(50).min(state.config.max_log_lines);

    let output = run_cmd(
        "journalctl",
        &[
            "-u", &query.unit,
            "-n", &lines.to_string(),
            "--no-pager",
            "-q",
        ],
    )
    .await
    .map_err(|e| format!("journalctl failed: {e}"))?;

    let logs: Vec<String> = output.lines().map(|l| l.to_string()).collect();

    Ok(Json(LogsResponse {
        unit: query.unit,
        lines: logs.len(),
        logs,
    }))
}
