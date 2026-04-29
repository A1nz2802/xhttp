use std::{
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
};

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct Worker {
    id: usize,
    join_handle: Option<JoinHandle<()>>,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
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

            workers.push(Worker {
                id,
                join_handle: Some(join_handle),
            });
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job: Job = Box::new(f);

        let sender = match self.sender.as_ref() {
            Some(s) => s,
            None => {
                eprintln!("Cannot execute job: thread pool is shutting down");
                return;
            }
        };

        if let Err(e) = sender.send(job) {
            eprintln!("Failed to send job to thread pool: {e}");
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(handle) = worker.join_handle.take() {
                if let Err(e) = handle.join() {
                    eprintln!("Worker {} panicked during shutdown: {:?}", worker.id, e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::time::Duration;

    #[test]
    fn executes_a_single_job() {
        let pool = ThreadPool::new(2);
        let counter = Arc::new(Mutex::new(0));

        let counter_clone = Arc::clone(&counter);
        pool.execute(move || {
            *counter_clone.lock().unwrap() += 1;
        });

        thread::sleep(Duration::from_millis(100));

        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[test]
    fn executes_many_jobs() {
        let pool = ThreadPool::new(2);
        let counter = Arc::new(Mutex::new(0));

        for _ in 0..10 {
            let counter_clone = Arc::clone(&counter);
            pool.execute(move || {
                *counter_clone.lock().unwrap() += 1;
            });
        }

        thread::sleep(Duration::from_millis(200));

        assert_eq!(*counter.lock().unwrap(), 10);
    }

    #[test]
    fn jobs_run_in_parallel() {
        let pool_size = 4;
        let pool = ThreadPool::new(pool_size);

        let barrier = Arc::new(Barrier::new(pool_size + 1));

        for _ in 0..pool_size {
            let barrier = Arc::clone(&barrier);
            pool.execute(move || {
                barrier.wait();
            });
        }

        barrier.wait();
    }
}
