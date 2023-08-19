use std::fs::File;
use std::fs::OpenOptions;

use std::io::prelude::*;
use std::io::{Seek, SeekFrom};

use std::time::Instant;

fn main() {
    // DEFINE ERRORS
    let write_error = "Error while writing to file";
    let create_error = "Error while creating file";
    let open_error = "Error while opening file";
    let read_error = "Error while reading from file";
    let seek_error = "Error while resetting file pointer position";

    // OPENING THE DEVICE FILE TO READ/WRITE TO
    let mut devfile = OpenOptions::new().read(true).write(true).open("/dev/mymem").expect(open_error);

    // CREATE RESULTS FILE TO PRINT TO
    let mut results = File::create("results").expect(create_error);

    // TEST WRITING 1 BYTE
    results.write(b"1B TEST WRITE |").expect(write_error);
    for _ in 0..10 {
        devfile.seek(SeekFrom::Start(0)).expect(seek_error);

        // PERFORM TEST BY TIMING WRITING 1 BYTE
        let start = Instant::now();
        devfile.write(b"A").expect(write_error);
        let duration = start.elapsed();

        results.write(format!(" {:?} |", duration).as_bytes()).expect(write_error);
    }
    results.write(b"\n").expect(write_error);

    // TEST READING 1 BYTE
    results.write(b"1B TEST READ  |").expect(write_error);
    for _ in 0..10 {
        // PERFORM TEST BY TIMING READING 1 BYTE
        devfile.seek(SeekFrom::Start(0)).expect(seek_error);
        let mut tmp = String::new();

        let start = Instant::now();
        devfile.read_to_string(&mut tmp).expect(read_error);
        let duration = start.elapsed();

        results.write(format!(" {:?} |", duration).as_bytes()).expect(write_error);
    }
    results.write(b"\n").expect(write_error);

    devfile.seek(SeekFrom::Start(0)).expect(seek_error);

    // PERFORM TESTS READING/WRITING LARGER AMOUNTS OF BYTES
    let sizes = [64, 1024, 65536, 524288];
    let characters = [b'B', b'C', b'D', b'E'];
    for i in 0..4 {
        // SET UP THE RESULTS FILE ACCORDINGLY
        if i == 0 {
            results.write(b"64B TEST      |").expect(write_error);
        } else if i == 1 {
            results.write(b"1kB TEST      |").expect(write_error);
        } else if i == 2 {
            results.write(b"64kB TEST     |").expect(write_error);
        } else if i == 3 {
            results.write(b"512kB TEST    |").expect(write_error);
        }

        // SET UP ARRAYS TO READ AND WRITE TO
        let tmp1 = String::from_utf8(vec![characters[i]; sizes[i]]).expect("error");
        let tmp2 = &tmp1[0..sizes[i]];
        let mut tmp3 = String::new();

        // RUN TESTS BY READING/WRITING TO THE FILES
        for _ in 0..10 {
            devfile.seek(SeekFrom::Start(0)).expect(seek_error);

            let start = Instant::now();
            devfile.write(tmp2.as_bytes()).expect(write_error);
            devfile.read_to_string(&mut tmp3).expect(read_error);
            let duration = start.elapsed();

            results.write(format!(" {:?} |", duration).as_bytes()).expect(write_error);
        }

        results.write(b"\n").expect(write_error);
    }
}
