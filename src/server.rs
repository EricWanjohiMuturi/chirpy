use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::SystemTime,
};

use apalis::prelude::BoxDynError;
use apalis_sqlite::SqlitePool;
use serde_json::json;
use tokio::sync::RwLock;

use crate::{
    cli::{Config, Mode, StorageConfig},
    http::run_http_server,
    job::Job,
    resp::{command::WorkerInfo, run_resp_server},
    storage::Storage,
};

pub struct Server<Mode> {
    pub(crate) mode: Mode,
    pub(crate) backend: Storage,
    pub(crate) workers: HashMap<String, WorkerInfo>,
    pub(crate) working_jobs: HashMap<String, String>,
    pub(crate) queues: HashMap<String, VecDeque<String>>,
    pub(crate) start_time: SystemTime,
}

impl<Mode> Server<Mode> {
    pub fn create(backend: Storage, mode: Mode) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            mode,
            backend,
            queues: HashMap::new(),
            working_jobs: HashMap::new(),
            workers: HashMap::new(),
            start_time: SystemTime::now(),
        }))
    }

    pub async fn push_job(&mut self, job: Job) -> Result<String, String> {
        self.backend.push_job(job).await
    }

    pub async fn fetch_job(&mut self, queues: &[String], wid: &String) -> Option<Job> {
        self.backend.fetch_job(queues, wid).await
    }

    pub async  fn ack_job(&mut self, jid: &str) -> Result<(), String> {
        let wid = self.working_jobs.get(jid).unwrap();
        self.backend.ack_job(jid, wid).await?;
        Ok(())
    }

    pub async fn fail_job(
        &mut self,
        jid: &str,
        err_type: &str,
        message: &str,
        backtrace: Vec<String>,
    ) -> Result<(), String> {
        let wid = self.working_jobs.get(jid).unwrap();
        let res = json!({
            "error_type": err_type,
            "message": message,
            "backtrace": backtrace
        });
        let res = serde_json::to_string(&res).unwrap();
        self.backend.fail_job(jid, wid, &res).await?;
        Ok(())
    }

    pub fn flush(&mut self) {
        // self.jobs.clear();
        self.queues.clear();
        // self.scheduled_jobs.clear();
        self.working_jobs.clear();
        // self.retries.clear();
        // self.dead_jobs.clear();
    }
}

pub async fn run_server(config: Config) -> Result<(), BoxDynError> {
    let storage = match config.storage {
        StorageConfig::Sqlite { url } => Storage::Sqlite(SqlitePool::connect_lazy(&url)?),
    };
    let mut handles = Vec::new();

    for mode in config.modes {
        match mode {
            Mode::Http(http) => {
                let server = Server::create(storage.clone(), http);
                let app = run_http_server(server);
                let handle = tokio::spawn(app);
                handles.push(handle);
            }
            Mode::Resp(resp) => {
                let server = Server::create(storage.clone(), resp);
                let app = run_resp_server(server);
                let handle = tokio::spawn(app);
                handles.push(handle);
            }
            Mode::Rpc(_) => {
                todo!();
            }
        }
    }

    futures::future::join_all(handles).await;

    Ok(())
}
