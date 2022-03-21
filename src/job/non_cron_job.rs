use crate::job::job_data::{JobStoredData, JobType};
use crate::job::{Job, JobToRunAsync};
use crate::job_store::JobStoreLocked;
use crate::{JobScheduler, JobSchedulerError, JobToRun};
use chrono::{DateTime, Utc};
use cron::Schedule;
use std::ops::Add;
use std::time::{Duration, SystemTime};
use tokio::sync::oneshot::Receiver;
use uuid::Uuid;

pub struct NonCronJob {
    pub run: Box<JobToRun>,
    pub run_async: Box<JobToRunAsync>,
    pub data: JobStoredData,
    pub async_job: bool,
}

impl Job for NonCronJob {
    fn is_cron_job(&self) -> bool {
        false
    }

    fn schedule(&self) -> Option<Schedule> {
        None
    }

    fn repeated_every(&self) -> Option<u64> {
        self.data.job.as_ref().and_then(|jt| match jt {
            crate::job::job_data::job_stored_data::Job::CronJob(_) => None,
            crate::job::job_data::job_stored_data::Job::NonCronJob(ncj) => Some(ncj.repeated_every),
        })
    }

    fn last_tick(&self) -> Option<DateTime<Utc>> {
        self.data.last_tick_utc()
    }

    fn set_last_tick(&mut self, tick: Option<DateTime<Utc>>) {
        self.data.set_last_tick(tick);
    }

    fn next_tick(&self) -> Option<DateTime<Utc>> {
        self.data.next_tick_utc()
    }

    fn set_next_tick(&mut self, tick: Option<DateTime<Utc>>) {
        self.data.set_next_tick(tick)
    }

    fn set_count(&mut self, count: u32) {
        self.data.count = count;
    }

    fn count(&self) -> u32 {
        self.data.count
    }

    fn increment_count(&mut self) {
        self.data.count = if self.data.count + 1 < u32::MAX {
            self.data.count + 1
        } else {
            0
        }; // Overflow check
    }

    fn job_id(&self) -> Uuid {
        self.data.id.as_ref().cloned().map(|e| e.into()).unwrap()
    }

    fn run(&mut self, jobs: JobScheduler) -> Receiver<bool> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let job_id = self.job_id();

        if !self.async_job {
            (self.run)(job_id, jobs);
            if let Err(e) = tx.send(true) {
                eprintln!("Error notifying done {:?}", e);
            }
        } else {
            let future = (self.run_async)(job_id, jobs);
            tokio::task::spawn(async move {
                future.await;
                if let Err(e) = tx.send(true) {
                    eprintln!("Error notifying done {:?}", e);
                }
            });
        }
        rx
    }

    fn job_type(&self) -> JobType {
        self.data.job_type()
    }

    fn ran(&self) -> bool {
        self.data.ran
    }

    fn set_ran(&mut self, ran: bool) {
        self.data.ran = ran;
    }

    fn stop(&self) -> bool {
        self.data.stopped
    }

    fn set_stopped(&mut self) {
        self.data.stopped = true;
    }

    fn set_started(&mut self) {
        self.data.stopped = false;
    }

    fn job_data_from_job_store(
        &mut self,
        job_store: JobStoreLocked,
    ) -> Result<Option<JobStoredData>, JobSchedulerError> {
        let mut job_store = job_store;
        let jd = job_store.get_job_data(&self.job_id())?;
        Ok(jd)
    }

    fn job_data_from_job(&mut self) -> Result<Option<JobStoredData>, JobSchedulerError> {
        Ok(Some(self.data.clone()))
    }

    fn set_job_data(&mut self, job_data: JobStoredData) -> Result<(), JobSchedulerError> {
        self.data = job_data;
        Ok(())
    }
}
