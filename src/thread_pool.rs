//! A fixed-size thread pool for executing jobs concurrently.
//!
//! Spawning an OS thread per request is unviable: each thread has its own
//! kernel-managed stack (~8 MiB on Linux by default) and creation/destruction
//! cost. The pool keeps a fixed number of worker threads alive and reuses
//! them across an unbounded number of jobs, which keeps memory and latency
//! predictable under load.

use std::{
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
};

/// A unit of work submitted to the pool.
///
/// The trait bounds capture every constraint required to send a closure
/// across thread boundaries safely:
///
/// - `FnOnce()`: a job runs exactly once and is consumed in the process.
/// - `Send`: the closure can be transferred to another thread.
/// - `'static`: the closure may not borrow data from the caller's stack,
///   because workers can outlive the function that submitted the job.
///
/// `Box<dyn ...>` is required because closures have anonymous, unsized
/// types; boxing erases the type and gives every job a uniform handle the
/// channel can carry.
type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct Worker {
    id: usize,
    // Wrapped in `Option` so `Drop` can take the handle out by value and
    // call `join()`, which consumes ownership. We only have `&mut self` in
    // `Drop::drop`, so moving the field directly is impossible — `take()`
    // swaps in `None` and returns the inner value.
    join_handle: Option<JoinHandle<()>>,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    // Wrapped in `Option` for the same reason as `Worker.join_handle`:
    // `Drop` needs to *drop* the sender (closing the channel) so that
    // workers blocked in `recv()` get an `Err` and exit their loops.
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    /// Creates a pool with `size` worker threads ready to receive jobs.
    ///
    /// Construction has three stages:
    ///
    /// 1. Open an mpsc channel — the inbox for `Job`s.
    /// 2. Wrap the `Receiver` in `Arc<Mutex<...>>` so multiple workers can
    ///    share it. The `Receiver` is not `Clone`, so direct sharing is
    ///    impossible; `Arc` provides shared ownership and `Mutex` ensures
    ///    only one worker calls `recv()` at a time.
    /// 3. Spawn `size` worker threads, each holding a clone of the `Arc`
    ///    and looping on `recv()` until the channel closes.
    pub fn new(size: usize) -> ThreadPool {
        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            // Each worker needs its own handle to the shared receiver.
            // `Arc::clone` is cheap — it bumps the atomic refcount; the
            // underlying `Mutex<Receiver<Job>>` is not duplicated.
            let receiver = Arc::clone(&receiver);

            // `move` transfers ownership of `receiver` and `id` into the
            // closure. Without it, the closure would only borrow them, and
            // the borrow checker rejects that because the spawned thread
            // can outlive the current scope.
            let join_handle = thread::spawn(move || {
                loop {
                    // The MutexGuard must be released *before* the job
                    // runs, otherwise the worker holds the lock for the
                    // entire job duration and serializes the whole pool
                    // (the "selfish worker" deadlock pattern). Confining
                    // `guard` to this inner block guarantees it is dropped
                    // — and the lock released — as soon as the `Job` value
                    // leaves the block.
                    let job: Job = {
                        let guard = match receiver.lock() {
                            Ok(g) => g,
                            // A poisoned mutex means another thread
                            // panicked while holding the lock. The shared
                            // state is potentially inconsistent, so the
                            // safest choice is to terminate this worker.
                            Err(e) => {
                                eprintln!("Worker {id}: mutex poisoned, shutting down: {e}");
                                return;
                            }
                        };

                        match guard.recv() {
                            Ok(job) => job,
                            // `Err` from `recv()` means every `Sender` has
                            // been dropped — the pool is shutting down and
                            // no further jobs can arrive.
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

    /// Submits a job to be executed by some worker.
    ///
    /// Takes `&self` rather than `&mut self` because `Sender` is designed
    /// to be used through shared references: its internal queue is
    /// thread-safe. This allows the pool to be wrapped in `Arc` and called
    /// from many places without coordinating exclusive access.
    ///
    /// `F` is generic instead of taking a `Job` directly so callers can
    /// write `pool.execute(|| { ... })` without manually boxing the
    /// closure. The bounds are identical to `Job`'s — `Box::new` performs
    /// the type erasure inside.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job: Job = Box::new(f);

        // Once `Drop` has run, `sender` is `None`. Submitting a job after
        // that is a no-op rather than a panic — being permissive here
        // avoids surprising failures during shutdown races.
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
    /// Performs an orderly shutdown when the pool goes out of scope.
    ///
    /// Order matters:
    ///
    /// 1. Drop the `Sender`. This closes the channel, causing every
    ///    worker's next `recv()` to return `Err` and exit its loop.
    /// 2. Join each worker. Each `join()` blocks until that worker's
    ///    current job (if any) finishes, guaranteeing no in-flight work
    ///    is killed mid-execution.
    ///
    /// Joining before dropping the sender would deadlock: workers would
    /// be parked on `recv()` waiting for jobs that will never arrive.
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            // Let-chains (Rust 2024) flatten the two `if let` checks
            // into a single block: the body runs only when the handle
            // exists *and* the join returned an error.
            if let Some(handle) = worker.join_handle.take()
                && let Err(e) = handle.join()
            {
                eprintln!("Worker {} panicked during shutdown: {:?}", worker.id, e);
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

    /// Proves workers run in parallel rather than serialized behind a
    /// shared lock. The barrier opens only when `pool_size + 1` threads
    /// reach it (the workers plus this test thread). If jobs ran one at a
    /// time, the first worker would block at the barrier and the rest
    /// would never get a chance to enter — the test would hang.
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
