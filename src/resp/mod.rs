use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use apalis::prelude::BoxDynError;
use clap::Args;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use crate::{
    resp::{
        command::{BeatResponse, Command, WorkerInfo},
        state::ClientState,
    },
    server::Server,
};

pub mod command;
mod state;

const RESP_SIMPLE_STRING: &str = "+";
const RESP_ERROR: &str = "-";
const RESP_BULK_STRING: &str = "$";
const RESP_NULL_BULK_STRING: &str = "$-1\r\n";

const DEFAULT_QUEUE: &str = "default";

#[derive(Args, Default, Clone, Builder, Debug, Serialize, Deserialize)]
#[builder(setter(into))]
pub struct Resp {
    host: String,
    port: u16,
    #[serde(default)]
    require_auth: bool,
    password: Option<String>,

    salt: String,
    #[serde(default = "default_iterations")]
    #[builder(default = "1735")]
    iterations: i32,
}

fn default_iterations() -> i32 {
    1735
}

pub type RespServer = Arc<RwLock<Server<Resp>>>;

impl Server<Resp> {
    fn verify_password(&self, pwdhash: &str) -> bool {
        if let Some(password) = &self.mode.password {
            let hash = format!("{}{}", password, self.mode.salt);
            for _ in 0..self.mode.iterations {
                let mut hasher = Sha256::new();
                hasher.update(hash.as_bytes());
                // hash = format!("{:x}", hasher.finalize());
            }
            hash == pwdhash
        } else {
            true
        }
    }

    fn get_info(&self) -> String {
        let queued_jobs: usize = self.queues.values().map(|q| q.len()).sum();
        let uptime = self
            .start_time
            .elapsed()
            .unwrap_or(Duration::from_secs(0))
            .as_secs();

        format!(
            "chirpy:\n  version: 1.0.0\n  uptime: {}\n  connections: {}\n  command_count: 0\n  job_count: {}\n  queued_count: {}\n  scheduled_count: {}\n  retries_count: {}\n  dead_count: {}\n  workers_count: {}\n  tasks_count: 0\n  processes_count: 0",
            uptime,
            self.workers.len(),
            0, // self.jobs.len(),
            queued_jobs,
            0, // self.scheduled_jobs.len(),
            0, // self.retries.len(),
            0, // self.dead_jobs.len(),
            self.workers.len()
        )
    }
}

pub async fn handle_resp_client(
    stream: TcpStream,
    server: RespServer,
) -> Result<(), Box<dyn std::error::Error>> {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let mut client_state = ClientState::NotIdentified;
    let mut client_wid = String::new();

    // Send initial greeting
    let greeting = {
        let server = server.read().await;
        if server.mode.require_auth {
            format!(
                "+HI {{\"v\":2,\"s\":\"{}\",\"i\":{}}}\r\n",
                server.mode.salt, server.mode.iterations
            )
        } else {
            "+HI {\"v\":2}\r\n".to_string()
        }
    };

    writer.write_all(greeting.as_bytes()).await?;

    while let Ok(bytes_read) = reader.read_line(&mut line).await {
        if bytes_read == 0 {
            break;
        }

        let response =
            process_resp_command(&line, &mut client_state, &mut client_wid, server.clone()).await?;
        writer.write_all(response.as_bytes()).await?;

        if client_state == ClientState::End {
            break;
        }

        line.clear();
    }

    if !client_wid.is_empty() {
        let mut server = server.write().await;
        server.workers.remove(&client_wid);
    }

    Ok(())
}

async fn process_resp_command(
    line: &str,
    client_state: &mut ClientState,
    client_wid: &mut String,
    server: RespServer,
) -> Result<String, Box<dyn std::error::Error>> {
    // Parse the command using the Command enum
    let command = match Command::from_line(line) {
        Ok(cmd) => cmd,
        Err(e) => return Ok(format!("{}{}\r\n", RESP_ERROR, e)),
    };

    match (&command, &client_state) {
        (
            Command::Hello {
                version,
                pwdhash,
                hostname,
                wid,
                pid,
                labels,
            },
            _,
        ) => {
            if *version != 2 {
                return Ok(format!("{}Protocol version must be 2\r\n", RESP_ERROR));
            }

            let mut server = server.write().await;
            if server.mode.require_auth {
                let hash = match pwdhash {
                    Some(h) => h,
                    None => return Ok(format!("{}Authentication required\r\n", RESP_ERROR)),
                };

                if !server.verify_password(hash) {
                    return Ok(format!("{}Invalid password\r\n", RESP_ERROR));
                }
            }

            if let (Some(hostname), Some(wid), Some(pid), Some(labels)) =
                (hostname, wid, pid, labels)
            {
                let worker_info = WorkerInfo {
                    hostname: hostname.clone(),
                    wid: wid.clone(),
                    pid: *pid,
                    labels: labels.clone(),
                    last_beat: SystemTime::now(),
                    state: ClientState::Consumer(wid.clone()),
                };

                server.workers.insert(wid.clone(), worker_info);
                *client_state = ClientState::Consumer(wid.clone());
                *client_wid = wid.clone();
            } else {
                *client_state = ClientState::Identified;
            }

            Ok(format!("{}OK\r\n", RESP_SIMPLE_STRING))
        }

        (Command::End, _) => {
            *client_state = ClientState::End;
            Ok(format!("{}OK\r\n", RESP_SIMPLE_STRING))
        }

        (_, ClientState::NotIdentified) => Ok(format!(
            "{}Not identified. Send HELLO first\r\n",
            RESP_ERROR
        )),

        (Command::Push { job }, _) => {
            let mut server = server.write().await;
            match server.push_job(job.clone()).await {
                Ok(_) => Ok(format!("{}OK\r\n", RESP_SIMPLE_STRING)),
                Err(e) => Ok(format!("{}{}\r\n", RESP_ERROR, e)),
            }
        }

        (Command::Flush, _) => {
            let mut server = server.write().await;
            server.flush();
            Ok(format!("{}OK\r\n", RESP_SIMPLE_STRING))
        }

        (Command::Info, _) => {
            let server = server.read().await;
            let info = server.get_info();
            Ok(format!(
                "{}{}\r\n{}\r\n",
                RESP_BULK_STRING,
                info.len(),
                info
            ))
        }

        (Command::Fetch { queues }, ClientState::Consumer(wid))
        | (Command::Fetch { queues }, ClientState::Quiet(wid)) => {
            let queues = if queues.is_empty() {
                vec![DEFAULT_QUEUE.to_string()]
            } else {
                queues.clone()
            };

            let mut server = server.write().await;
            // server.process_scheduled_jobs();
            // server.process_retries();
            // server.check_expired_jobs();

            if matches!(client_state, ClientState::Quiet(_)) {
                return Ok(RESP_NULL_BULK_STRING.to_string());
            }

            if let Some(job) = server.fetch_job(&queues, wid).await {
                let jid = job.jid.clone();

                if let None = jid {
                    return Ok(RESP_NULL_BULK_STRING.to_string());
                }
                let wid = match client_state {
                    ClientState::Consumer(wid) => wid.clone(),
                    ClientState::Quiet(wid) => wid.clone(),
                    _ => unreachable!(),
                };

                server.working_jobs.insert(jid.unwrap(), wid);

                let job_json = serde_json::to_string(&job)?;
                Ok(format!(
                    "{}{}\r\n{}\r\n",
                    RESP_BULK_STRING,
                    job_json.len(),
                    job_json
                ))
            } else {
                Ok(RESP_NULL_BULK_STRING.to_string())
            }
        }

        (Command::Ack { jid }, ClientState::Consumer(_))
        | (Command::Ack { jid }, ClientState::Quiet(_))
        | (Command::Ack { jid }, ClientState::Terminating(_)) => {
            let mut server = server.write().await;
            match server.ack_job(jid).await {
                Ok(_) => Ok(format!("{}OK\r\n", RESP_SIMPLE_STRING)),
                Err(e) => Ok(format!("{}{}\r\n", RESP_ERROR, e)),
            }
        }

        (
            Command::Fail {
                jid,
                errtype,
                message,
                backtrace,
            },
            ClientState::Consumer(_),
        )
        | (
            Command::Fail {
                jid,
                errtype,
                message,
                backtrace,
            },
            ClientState::Quiet(_),
        )
        | (
            Command::Fail {
                jid,
                errtype,
                message,
                backtrace,
            },
            ClientState::Terminating(_),
        ) => {
            let mut server = server.write().await;
            match server.fail_job(jid, errtype, message, backtrace.clone()).await {
                Ok(_) => Ok(format!("{}OK\r\n", RESP_SIMPLE_STRING)),
                Err(e) => Ok(format!("{}{}\r\n", RESP_ERROR, e)),
            }
        }

        (
            Command::Beat {
                wid,
                current_state,
                rss_kb: _,
            },
            ClientState::Consumer(_),
        )
        | (
            Command::Beat {
                wid,
                current_state,
                rss_kb: _,
            },
            ClientState::Quiet(_),
        )
        | (
            Command::Beat {
                wid,
                current_state,
                rss_kb: _,
            },
            ClientState::Terminating(_),
        ) => {
            let state_change = {
                let mut server = server.write().await;
                if let Some(worker) = server.workers.get_mut(wid) {
                    worker.last_beat = SystemTime::now();

                    if let Some(state) = current_state {
                        match state.as_str() {
                            "quiet" => {
                                worker.state = ClientState::Quiet(wid.clone());
                                *client_state = ClientState::Quiet(wid.clone());
                            }
                            "terminate" => {
                                worker.state = ClientState::Terminating(wid.clone());
                                *client_state = ClientState::Terminating(wid.clone());
                            }
                            _ => {}
                        }
                    }

                    match worker.state {
                        ClientState::Quiet(_) => Some("quiet"),
                        ClientState::Terminating(_) => Some("terminate"),
                        _ => None,
                    }
                } else {
                    None
                }
            };

            if let Some(new_state) = state_change {
                let response = BeatResponse {
                    state: new_state.to_string(),
                };
                let response_json = serde_json::to_string(&response)?;
                Ok(format!("{}{}\r\n", RESP_SIMPLE_STRING, response_json))
            } else {
                Ok(format!("{}OK\r\n", RESP_SIMPLE_STRING))
            }
        }

        (Command::Fetch { .. }, _)
        | (Command::Ack { .. }, _)
        | (Command::Fail { .. }, _)
        | (Command::Beat { .. }, _) => Ok(format!(
            "{}Command not allowed in current state\r\n",
            RESP_ERROR
        )),

        (Command::Hi { .. }, _) => Ok(format!("{}HI is a server-only command\r\n", RESP_ERROR)),
    }
}

pub async fn run_resp_server(server: RespServer) -> Result<(), BoxDynError> {
    let listener = {
        let server = server.read().await;
        let listener =
            TcpListener::bind(format!("{}:{}", server.mode.host, server.mode.port)).await?;
        println!("RESP server listening on port {}", server.mode.port);
        listener
    };

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("New RESP connection from {}", addr);
                let server_clone = server.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_resp_client(stream, server_clone).await {
                        eprintln!("Error handling RESP client: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Error accepting RESP connection: {}", e);
            }
        }
    }
}
