//! mymemthread module in Rust.

use kernel::prelude::*;
use kernel::sync::{CondVar, Mutex};
use kernel::task::Task;
use kernel::bindings::*;
use kernel::pr_cont;

module! {
    type: MyMemThread,
    name: "mymemthread",
    author: "Jakub Bester",
    description: "MyMemThread test module",
    license: "GPL",
}

struct MyMemThread;

// HELPER FUNCTIONS
//   timing function
//   push string function

fn time() -> timespec64 {
    let mut ts: timespec64 = timespec64 {
        tv_sec: 0,
        tv_nsec: 0,
    };
    
    unsafe {
        ktime_get_ts64(&mut ts as *mut timespec64);
    }

    return ts;
}

fn test() {
    // INITIALIZING WRITE/READ INTERFACE
    let mut instance = mymem::MyMem;

    // TEST WRITING 1 BYTE
    pr_info!("\n\n1B TEST WRITE | ");
    for _ in 0..10 {
        // PERFORM TEST BY TIMING WRITING 1 BYTE
        let ts_s = time();
        instance.write(b"A", 0);
        let ts_e = time();
        
        pr_cont!("{:5} | ", ts_e.tv_nsec - ts_s.tv_nsec);
    }
    pr_cont!("\n");

    // TEST READING 1 BYTE
    let mut tmp:[u8; 1] = [0];
    pr_cont!("1B TEST READ  | ");
    for _ in 0..10 {
        // PERFORM TEST BY TIMING READING 1 BYTE
        let ts_s = time();
        instance.read(&mut tmp, 0);
        let ts_e = time();

        pr_cont!("{:5} | ", ts_e.tv_nsec - ts_s.tv_nsec);
    }
    pr_cont!("\n");

    // PERFORM TESTS READING/WRITING LARGER AMOUNTS OF BYTES
    for i in 0..4 {
        // SET UP THE RESULTS FILE ACCORDINGLY
        if i == 0 {
            pr_cont!("64B TEST      | ");
        } else if i == 1 {
            pr_cont!("1kB TEST      | ");
        }

        // RUN TESTS BY READING/WRITING TO THE FILES
        for _ in 0..10 {
            if i == 0 {
                let mut tmp64_1:[u8; 64] = [b'B'; 64];
                let ts_s = time();
                instance.write(&mut tmp64_1, 0);
                instance.read(&mut tmp64_1, 0);
                let ts_e = time();
                pr_cont!("{:5} | ", ts_e.tv_nsec - ts_s.tv_nsec);
            } else if i == 1 {
                let mut tmp1024_1:[u8; 1024] = [b'C'; 1024];
                let ts_s = time();
                instance.write(&mut tmp1024_1, 0);
                instance.read(&mut tmp1024_1, 0);
                let ts_e = time();
                pr_cont!("{:5} | ", ts_e.tv_nsec - ts_s.tv_nsec);
            } // unable to do higher amounts due to page fault occuring
        }
        pr_cont!("\n");
    }
}

fn threadfn() {
    let mut instance = mymem::MyMem;

    // DISPLAY THE THREAD THAT IT'S BE RUN ON
    // pr_info!("Running from thread {}", Task::current().pid());

    // COUNT DOWN TO ENSURE THAT PROPER NUMBER OF THREADS IS SPAWNED
    let mut guard = COUNT.lock();
    *guard -= 1;
    if *guard == 0 {
        COUNT_IS_ZERO.notify_all();
    }

    // PERFORM THE DATA RACE TEST
    const NUMBER:u64 = 200;
    let mut buf:[u8; 8] = [0; 8];

    for _ in 0..NUMBER {
        instance.read(&mut buf, 0); // read the string and store in temporary storage

        let mut val:u64 = u64::from_ne_bytes(buf[..].try_into().unwrap()); // convert array into u64 integer
        val += 1;
        
        buf = val.to_ne_bytes(); // convert integer into array of bytes

        instance.write(&buf, 0);
    }
}

fn thread() {
    const WORKERS:u64 = 50;

    // SPAWN DIFFERENT THREADS
    *COUNT.lock() = WORKERS;
    for i in 0..WORKERS {
        Task::spawn(fmt!("test{i}"), threadfn).unwrap();
    }

    // WAIT FOR COUNT TO DROP TO ZERO
    let mut guard = COUNT.lock();
    while *guard != 0 {
        let _wait:bool = COUNT_IS_ZERO.wait(&mut guard);
    }
}

kernel::init_static_sync! {
    static COUNT: Mutex<u64> = 0;
    static COUNT_IS_ZERO: CondVar;
}

// IMPLEMENTING THE OPERATIONS THAT DEFINE THE MODULE

impl kernel::Module for MyMemThread {
    fn init(_name: &'static CStr, _module: &'static ThisModule) -> Result<Self> {
        pr_info!("Loading in the MyMemThread module!");

        let mut instance = mymem::MyMem;

        // TEST PART #2 USING THE MYMEM MODULE
        pr_info!("Running Test from Part #2"); test();

        // TEST PART #3 USING THE MYMEM MODULE
        instance.write(b"DEADBEEF", 0);
        pr_info!("Running Test from Part #3"); thread();
        
        pr_info!("Number (at Start): {}", u64::from_ne_bytes(*b"DEADBEEF"));

        let mut buf:[u8; 8] = [0; 8];
        instance.read(&mut buf, 0);
        pr_info!("Number (at End): {}\n", u64::from_ne_bytes(buf[..].try_into().unwrap()));

        Ok(MyMemThread)
    }
}

impl Drop for MyMemThread {
    fn drop(&mut self) {
        // CLOSING OUT THE MODULE
        pr_info!("Closing the MyMemThread module!\n");
    }
}
