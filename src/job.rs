use apalis_sqlite::SqliteTask;
use serde::{Deserialize, Serialize};

/// Job structure as defined in the protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Job {
    pub jid: Option<String>,
    pub jobtype: String,
    pub args: Vec<serde_json::Value>,
    #[serde(default = "default_queue")]
    pub queue: String,
    #[serde(default = "default_reserve_for")]
    pub reserve_for: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    #[serde(default = "default_retry")]
    pub retry: i32,
    #[serde(default)]
    pub backtrace: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enqueued_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<Failure>,
}

impl TryFrom<SqliteTask<Vec<u8>>> for Job {
    type Error = serde_json::Error;
    fn try_from(value: SqliteTask<Vec<u8>>) -> Result<Self, Self::Error> {
        Ok(Job {
            jid: value.parts.task_id.as_ref().map(ToString::to_string),
            jobtype: value.parts.ctx.queue().as_ref().cloned().unwrap(),
            args: serde_json::from_slice(&value.args)?,
            queue: value.parts.ctx.queue().as_ref().cloned().unwrap(),
            reserve_for: default_reserve_for(),
            at: Some(value.parts.run_at.to_string()),
            retry: default_retry(),
            backtrace: 0,
            created_at: Some(value.parts.run_at.to_string()),
            custom: Some(value.parts.ctx.meta().clone().into()),
            enqueued_at: None,
            failure: None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Failure {
    pub failed_at: String,
    pub retry_count: i32,
    pub err_type: String,
    pub message: String,
    pub backtrace: Vec<String>,
}

fn default_queue() -> String {
    "default".to_string()
}

fn default_reserve_for() -> u64 {
    1800
}

fn default_retry() -> i32 {
    25
}
