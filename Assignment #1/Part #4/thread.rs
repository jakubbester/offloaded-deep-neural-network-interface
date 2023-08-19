use std::fs::OpenOptions;

use std::io::prelude::*;
use std::io::{Seek, SeekFrom};
use std::convert::TryInto;

// INCLUDE THREADING TOOLS
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // DEFINE ERRORS
    let write_error = "Error while writing to file";
    let open_error = "Error while opening file";
    let read_error = "Error while reading from file";
    let seek_error = "Error while resetting file pointer position";
    let join_error = "Error while joining threads";
    
     // OPENING THE DEVICE FILE TO READ/WRITE TO
    let mut devfile = OpenOptions::new().read(true).write(true).open("/dev/mymem").expect(open_error);

    // WRITE THE INITIAL BYTES TO THE FILE
    devfile.write(b"DEADBEEF").expect(write_error);

    // CREATING THE REQUIRED THREADS
    const WORKERS:u64 = 50;
    const NUMBER:u64 = 200;

    // PRINT THE INITIAL RESULT
    let mut buf : [u8; 8] = [0; 8];
    devfile.read(&mut buf).expect(read_error); // read the string in the buffer

    let val:u64 = u64::from_ne_bytes(buf[..].try_into().unwrap()); // convert array into u64 integer
    println!("{}", val);

    // PERFORM MANIPULATION OF SHARED MEMORY
    let file = Arc::new(Mutex::new(devfile));
    let mut handles = vec![];

    for _ in 0..WORKERS {
        let file = Arc::clone(&file);

        let handle = thread::spawn(move || {
            let mut file = file.lock().unwrap();
            let mut buf : [u8; 8] = [0; 8];

            for _ in 0..NUMBER {
                file.seek(SeekFrom::Start(0)).expect(seek_error);
                file.read(&mut buf).expect(read_error); // read the string and store in temporary storage

                let mut val:u64 = u64::from_ne_bytes(buf[..].try_into().unwrap()); // convert array into u64 integer
                val += 1;
                
                buf = val.to_ne_bytes(); // convert integer into array of bytes

                file.seek(SeekFrom::Start(0)).expect(seek_error);
                file.write(&buf).expect(write_error);
            }

        });

        handles.push(handle);
    }

    // JOIN ALL OF THE THREADS TOGETHER (MAIN WAITS FOR THEM)
    for handle in handles {
        handle.join().expect(join_error);
    }

    // PERFORM FINAL READING
    let file = Arc::clone(&file);
    let mut file = file.lock().unwrap();

    let mut buf : [u8; 8] = [0; 8];
    file.read(&mut buf).expect(read_error); // read the string into the buffer

    let val:u64 = u64::from_ne_bytes(buf[..].try_into().unwrap()); // convert array into u64 integer

    // PRINT THE FINAL RESULT
    println!("{}", val);
}
