use std::str::FromStr;

use apalis::prelude::{TaskBuilder, TaskId, WorkerContext};
use apalis_sqlite::{Config, SqlitePool};
use futures::FutureExt;
use ulid::Ulid;

use crate::job::Job;

#[derive(Debug, Clone)]
pub enum Storage {
    Sqlite(SqlitePool),
}

impl Storage {
    pub async fn push_job(&mut self, job: Job) -> Result<String, String> {
        match self {
            Storage::Sqlite(pool) => {
                let task_id = job
                    .jid
                    .map(|a| TaskId::from_str(&a).ok())
                    .flatten()
                    .unwrap_or(TaskId::new(Ulid::new()));
                let task = TaskBuilder::new(serde_json::to_vec(&job.args).unwrap())
                    .with_task_id(task_id.clone())
                    .build();
                let config = Config::new(&job.queue);
                apalis_sqlite::sink::push_tasks(pool.clone(), config, vec![task])
                    .await
                    .unwrap();
                return Ok(task_id.to_string());
            }
        }
    }
    pub async fn fetch_job(&mut self, queues: &[String], wid: &String) -> Option<Job> {
        match self {
            Storage::Sqlite(pool) => {
                let backends = queues
                    .iter()
                    .map(|queue| {
                        let config = Config::new(queue).set_buffer_size(1);
                        apalis_sqlite::fetcher::fetch_next(
                            pool.clone(),
                            config,
                            WorkerContext::new::<Self>(wid),
                        )
                        .boxed()
                    })
                    .collect::<Vec<_>>();
                let (res, _index, _remaining) = futures::future::select_all(backends).await;
                let job = res.map(|s| s[0].clone()).ok();
                job.map(|j| j.try_into().ok()).flatten()
            }
        }
    }

    // ack_job

    // fail_job
}
