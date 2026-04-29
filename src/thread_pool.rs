use std::{
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
};

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct Worker {
    id: usize,
    join_handle: JoinHandle<()>,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            let receiver = Arc::clone(&receiver);

            let join_handle = thread::spawn(move || {
                loop {
                    let job: Job = {
                        let guard = match receiver.lock() {
                            Ok(g) => g,
                            Err(e) => {
                                eprintln!("Worker {id}: mutex poisoned, shutting down: {e}");
                                return;
                            }
                        };

                        match guard.recv() {
                            Ok(job) => job,
                            Err(_) => {
                                println!("Worker {id}: channel closed, shutting down");
                                return;
                            }
                        }
                    };

                    println!("Worker {id}: executing job");
                    job();
                }
            });

            workers.push(Worker { id, join_handle });
        }

        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job: Job = Box::new(f);

        if let Err(e) = self.sender.send(job) {
            eprintln!("Failed to send job to thread pool: {e}");
        }
    }
}
