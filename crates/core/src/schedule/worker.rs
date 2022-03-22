use std::{
    sync::{mpsc::Receiver, Arc, Mutex, RwLock},
    thread::{self, JoinHandle},
};

use crate::{Job, JobHandlerRw, Scheduler};

pub struct Worker {
    scheduler: Arc<RwLock<Scheduler>>,
    thread_handle: Option<JoinHandle<bool>>,
}

impl Default for Worker {
    fn default() -> Self {
        Self {
            scheduler: Arc::new(RwLock::new(Scheduler::new())),
            thread_handle: None,
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.scheduler.write().unwrap().uninit();
    }
}

impl Worker {
    pub fn is_started(&self) -> bool {
        self.thread_handle.is_some()
    }
    pub fn get_job(receiver: &Arc<Mutex<Receiver<Job>>>) -> Option<Job> {
        let recv = receiver.lock().unwrap();
        if let Ok(job) = recv.try_recv() {
            return Some(job);
        }
        None
    }

    pub fn start(
        &mut self,
        name: &str,
        job_handler: &JobHandlerRw,
        job_receiver: Arc<Mutex<Receiver<Job>>>,
    ) {
        if self.thread_handle.is_none() {
            let builder = thread::Builder::new().name(name.into());
            let scheduler = Arc::clone(&self.scheduler);
            let job_handler = job_handler.clone();
            let t = builder
                .spawn(move || {
                    inox_profiler::register_thread!();
                    scheduler.write().unwrap().resume();
                    loop {
                        let can_continue =
                            scheduler
                                .write()
                                .unwrap()
                                .run_once(true, &job_handler, &job_receiver);
                        while let Some(job) = Worker::get_job(&job_receiver) {
                            job.execute();
                        }
                        if !can_continue {
                            return false;
                        }
                    }
                })
                .unwrap();
            self.thread_handle = Some(t);
        }
    }

    pub fn stop(&mut self) {
        if self.thread_handle.is_some() {
            let t = self.thread_handle.take().unwrap();

            self.scheduler.write().unwrap().cancel();
            t.join().unwrap();

            self.thread_handle = None;
        }
    }
}
