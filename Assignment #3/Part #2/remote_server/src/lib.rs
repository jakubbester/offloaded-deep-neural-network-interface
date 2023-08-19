/// Implemented using the Let's Get Rusty YouTube tutorial
/// "Building a Web Server in Rust" Parts #1 - #3

use std::{sync::{Arc, Mutex, mpsc}, thread};

/// CREATE A POOL OF THREADS TO BE USED

pub struct ThreadPool {
    _workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The 'new' function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut _workers = Vec::with_capacity(size);

        for id in 0..size {
            _workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { _workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }
}

/// IMPLEMENT WORKER/THREAD CAPABILITY TO EXECUTE ONE PINGED TASK

struct Worker {
    _id: usize,
    _thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(_id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let _thread = thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv().unwrap();

            // println!("Worker {} got a job; executing.", id);

            job();
        });

        Worker { _id, _thread }
    }
}
