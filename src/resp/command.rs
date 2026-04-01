use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::{job::Job, resp::state::ClientState};

#[allow(unused)]
/// Represents all Protocol commands
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Initial greeting from server to client
    /// Sent automatically when client connects
    Hi {
        version: i32,
        salt: Option<String>,
        iterations: Option<i32>,
    },

    /// Client authentication and identification
    /// Must be first command from client
    Hello {
        version: i32,
        pwdhash: Option<String>,
        // Producer-only fields (all None)
        // Worker-specific fields (all Some for workers)
        hostname: Option<String>,
        wid: Option<String>,
        pid: Option<i32>,
        labels: Option<Vec<String>>,
    },

    /// Push a new job to the server (Producer command)
    Push { job: Job },

    /// Fetch a job for execution (Consumer command)
    Fetch { queues: Vec<String> },

    /// Acknowledge successful job completion (Consumer command)
    Ack { jid: String },

    /// Report job failure (Consumer command)
    Fail {
        jid: String,
        errtype: String,
        message: String,
        backtrace: Vec<String>,
    },

    /// Worker heartbeat (Consumer command)
    Beat {
        wid: String,
        current_state: Option<String>,
        rss_kb: Option<i32>,
    },

    /// Request server information
    Info,

    /// Clear all data from server
    Flush,

    /// Terminate connection gracefully
    End,
}

#[allow(unused)]
impl Command {
    /// Parse a command from a protocol line
    pub fn from_line(line: &str) -> Result<Self, CommandParseError> {
        let line = line.trim();
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        let verb = parts[0].to_uppercase();
        let args = parts.get(1).copied().unwrap_or("");

        match verb.as_str() {
            "HELLO" => {
                let hello: HelloData = serde_json::from_str(args)
                    .map_err(|e| CommandParseError::InvalidJson(e.to_string()))?;

                Ok(Command::Hello {
                    version: hello.v,
                    pwdhash: hello.pwdhash,
                    hostname: hello.hostname,
                    wid: hello.wid,
                    pid: hello.pid,
                    labels: hello.labels,
                })
            }
            "PUSH" => {
                let job: Job = serde_json::from_str(args)
                    .map_err(|e| CommandParseError::InvalidJson(e.to_string()))?;
                Ok(Command::Push { job })
            }
            "FETCH" => {
                let queues = if args.is_empty() {
                    vec![]
                } else {
                    serde_json::from_str::<Vec<String>>(args).unwrap_or_else(|_| vec![])
                };
                Ok(Command::Fetch { queues })
            }
            "ACK" => {
                let ack: AckData = serde_json::from_str(args)
                    .map_err(|e| CommandParseError::InvalidJson(e.to_string()))?;
                Ok(Command::Ack { jid: ack.jid })
            }
            "FAIL" => {
                let fail: FailData = serde_json::from_str(args)
                    .map_err(|e| CommandParseError::InvalidJson(e.to_string()))?;
                Ok(Command::Fail {
                    jid: fail.jid,
                    errtype: fail.errtype,
                    message: fail.message,
                    backtrace: fail.backtrace,
                })
            }
            "BEAT" => {
                let beat: BeatData = serde_json::from_str(args)
                    .map_err(|e| CommandParseError::InvalidJson(e.to_string()))?;
                Ok(Command::Beat {
                    wid: beat.wid,
                    current_state: beat.current_state,
                    rss_kb: beat.rss_kb,
                })
            }
            "INFO" => Ok(Command::Info),
            "FLUSH" => Ok(Command::Flush),
            "END" => Ok(Command::End),
            _ => Err(CommandParseError::UnknownCommand(verb)),
        }
    }

    /// Convert command to protocol line format
    pub fn to_line(&self) -> String {
        match self {
            Command::Hi {
                version,
                salt,
                iterations,
            } => {
                let mut data = serde_json::json!({ "v": version });
                if let Some(s) = salt {
                    data["s"] = serde_json::json!(s);
                }
                if let Some(i) = iterations {
                    data["i"] = serde_json::json!(i);
                }
                format!("HI {}", data)
            }
            Command::Hello {
                version,
                pwdhash,
                hostname,
                wid,
                pid,
                labels,
            } => {
                let data = HelloData {
                    v: *version,
                    pwdhash: pwdhash.clone(),
                    hostname: hostname.clone(),
                    wid: wid.clone(),
                    pid: *pid,
                    labels: labels.clone(),
                };
                format!("HELLO {}", serde_json::to_string(&data).unwrap())
            }
            Command::Push { job } => {
                format!("PUSH {}", serde_json::to_string(job).unwrap())
            }
            Command::Fetch { queues } => {
                if queues.is_empty() {
                    "FETCH".to_string()
                } else {
                    format!("FETCH {}", serde_json::to_string(queues).unwrap())
                }
            }
            Command::Ack { jid } => {
                format!("ACK {}", serde_json::json!({ "jid": jid }))
            }
            Command::Fail {
                jid,
                errtype,
                message,
                backtrace,
            } => {
                let data = FailData {
                    jid: jid.clone(),
                    errtype: errtype.clone(),
                    message: message.clone(),
                    backtrace: backtrace.clone(),
                };
                format!("FAIL {}", serde_json::to_string(&data).unwrap())
            }
            Command::Beat {
                wid,
                current_state,
                rss_kb,
            } => {
                let data = BeatData {
                    wid: wid.clone(),
                    current_state: current_state.clone(),
                    rss_kb: *rss_kb,
                };
                format!("BEAT {}", serde_json::to_string(&data).unwrap())
            }
            Command::Info => "INFO".to_string(),
            Command::Flush => "FLUSH".to_string(),
            Command::End => "END".to_string(),
        }
    }

    /// Returns true if this command is only valid for consumers
    pub fn is_consumer_only(&self) -> bool {
        matches!(
            self,
            Command::Fetch { .. }
                | Command::Ack { .. }
                | Command::Fail { .. }
                | Command::Beat { .. }
        )
    }

    /// Returns true if this command is only valid for producers
    pub fn is_producer_only(&self) -> bool {
        matches!(self, Command::Push { .. })
    }

    /// Returns true if this command can be used by both producers and consumers
    pub fn is_common(&self) -> bool {
        matches!(
            self,
            Command::Hello { .. } | Command::Info | Command::Flush | Command::End
        )
    }
}

// Helper structs for serialization/deserialization
#[derive(Debug, Serialize, Deserialize)]
struct HelloData {
    v: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pwdhash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pid: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AckData {
    jid: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FailData {
    jid: String,
    errtype: String,
    message: String,
    backtrace: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BeatData {
    wid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rss_kb: Option<i32>,
}

/// Errors that can occur when parsing commands
#[derive(Debug, Clone, PartialEq)]
pub enum CommandParseError {
    UnknownCommand(String),
    InvalidJson(String),
    #[allow(unused)]
    MissingArguments,
}

impl std::fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandParseError::UnknownCommand(cmd) => {
                write!(f, "Unknown command: {}", cmd)
            }
            CommandParseError::InvalidJson(err) => {
                write!(f, "Invalid JSON: {}", err)
            }
            CommandParseError::MissingArguments => {
                write!(f, "Missing required arguments")
            }
        }
    }
}

impl std::error::Error for CommandParseError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatResponse {
    pub state: String,
}

#[derive(Debug, Clone)]
pub struct WorkerInfo {
    pub hostname: String,
    pub wid: String,
    pub pid: i32,
    pub labels: Vec<String>,
    pub last_beat: SystemTime,
    pub state: ClientState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hello_producer() {
        let line = r#"HELLO {"v":2}"#;
        let cmd = Command::from_line(line).unwrap();

        match cmd {
            Command::Hello {
                version, hostname, ..
            } => {
                assert_eq!(version, 2);
                assert_eq!(hostname, None);
            }
            _ => panic!("Expected Hello command"),
        }
    }

    #[test]
    fn test_parse_hello_consumer() {
        let line =
            r#"HELLO {"v":2,"hostname":"localhost","wid":"worker1","pid":1234,"labels":["rust"]}"#;
        let cmd = Command::from_line(line).unwrap();

        match cmd {
            Command::Hello {
                version,
                hostname,
                wid,
                pid,
                labels,
                ..
            } => {
                assert_eq!(version, 2);
                assert_eq!(hostname, Some("localhost".to_string()));
                assert_eq!(wid, Some("worker1".to_string()));
                assert_eq!(pid, Some(1234));
                assert_eq!(labels, Some(vec!["rust".to_string()]));
            }
            _ => panic!("Expected Hello command"),
        }
    }

    #[test]
    fn test_parse_push() {
        let line = r#"PUSH {"jid":"123","jobtype":"test","args":["arg1"]}"#;
        let cmd = Command::from_line(line).unwrap();

        match cmd {
            Command::Push { job } => {
                assert_eq!(job.jid, "123");
                assert_eq!(job.jobtype, "test");
            }
            _ => panic!("Expected Push command"),
        }
    }

    #[test]
    fn test_parse_fetch() {
        let line = r#"FETCH ["queue1","queue2"]"#;
        let cmd = Command::from_line(line).unwrap();

        match cmd {
            Command::Fetch { queues } => {
                assert_eq!(queues, vec!["queue1", "queue2"]);
            }
            _ => panic!("Expected Fetch command"),
        }
    }

    #[test]
    fn test_parse_ack() {
        let line = r#"ACK {"jid":"123"}"#;
        let cmd = Command::from_line(line).unwrap();

        match cmd {
            Command::Ack { jid } => {
                assert_eq!(jid, "123");
            }
            _ => panic!("Expected Ack command"),
        }
    }

    #[test]
    fn test_parse_fail() {
        let line = r#"FAIL {"jid":"123","errtype":"RuntimeError","message":"failed","backtrace":["line1"]}"#;
        let cmd = Command::from_line(line).unwrap();

        match cmd {
            Command::Fail {
                jid,
                errtype,
                message,
                backtrace,
            } => {
                assert_eq!(jid, "123");
                assert_eq!(errtype, "RuntimeError");
                assert_eq!(message, "failed");
                assert_eq!(backtrace, vec!["line1"]);
            }
            _ => panic!("Expected Fail command"),
        }
    }

    #[test]
    fn test_parse_beat() {
        let line = r#"BEAT {"wid":"worker1","rss_kb":1024}"#;
        let cmd = Command::from_line(line).unwrap();

        match cmd {
            Command::Beat {
                wid,
                current_state,
                rss_kb,
            } => {
                assert_eq!(wid, "worker1");
                assert_eq!(current_state, None);
                assert_eq!(rss_kb, Some(1024));
            }
            _ => panic!("Expected Beat command"),
        }
    }

    #[test]
    fn test_parse_simple_commands() {
        assert!(matches!(Command::from_line("INFO").unwrap(), Command::Info));
        assert!(matches!(
            Command::from_line("FLUSH").unwrap(),
            Command::Flush
        ));
        assert!(matches!(Command::from_line("END").unwrap(), Command::End));
    }

    #[test]
    fn test_to_line() {
        let cmd = Command::Push {
            job: Job {
                jid: "123".to_string(),
                jobtype: "test".to_string(),
                args: vec![serde_json::json!("arg1")],
                queue: "default".to_string(),
                reserve_for: 1800,
                at: None,
                retry: 25,
                backtrace: 0,
                created_at: None,
                custom: None,
                enqueued_at: None,
                failure: None,
            },
        };

        let line = cmd.to_line();
        assert!(line.starts_with("PUSH "));
        assert!(line.contains("\"jid\":\"123\""));
    }

    #[test]
    fn test_command_type_checks() {
        let fetch = Command::Fetch { queues: vec![] };
        assert!(fetch.is_consumer_only());
        assert!(!fetch.is_producer_only());
        assert!(!fetch.is_common());

        let push = Command::Push {
            job: Job {
                jid: "123".to_string(),
                jobtype: "test".to_string(),
                args: vec![],
                queue: "default".to_string(),
                reserve_for: 1800,
                at: None,
                retry: 25,
                backtrace: 0,
                created_at: None,
                custom: None,
                enqueued_at: None,
                failure: None,
            },
        };
        assert!(!push.is_consumer_only());
        assert!(push.is_producer_only());
        assert!(!push.is_common());

        let info = Command::Info;
        assert!(!info.is_consumer_only());
        assert!(!info.is_producer_only());
        assert!(info.is_common());
    }
}
