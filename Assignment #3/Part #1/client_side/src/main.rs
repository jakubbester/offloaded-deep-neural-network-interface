extern crate server_side;
use server_side::*;

use std::sync::{Arc, Mutex};

use tflitec::interpreter::{Interpreter, Options};

// can comment out accordingly to get desired result

fn main() {
    // ANNOTATE THE VIDEO
    annotate(true);

    // CHANGE THE KEY TO END STREAMING
    terminate(97);

    // SETTING DELAY BETWEEN READ/WRITE
    //      this works best in conjunction with the ping call
    //      and adjusting the delay to be similar to that of the
    //      connection that is established between the client/server side
    //      and the remote server
    delay(0);

    println!("SETTING UP INTERPRETER ... \n");

    // LOADING THE MODEL/INTERPRETER
    let path = format!("resource/model_local.tflite");

	let interpreter = Interpreter::with_model_path(&path, Some(Options::default())).expect("Load model [FAILED]");
	interpreter.allocate_tensors().expect("Allocate tensors [FAILED]");

    let interpreter = Arc::new(Mutex::new(interpreter)); // CREATE A MUTEXED ATOMIC REFERENCE TO THE INTERPRETER
    let interpreter = Arc::clone(&interpreter);

    // DISPLAY THE FEED
    display(interpreter);
}
