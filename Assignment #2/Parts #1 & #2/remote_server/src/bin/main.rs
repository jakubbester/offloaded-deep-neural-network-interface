/// Setting up the server to receive connections/stream data from user (using ThreadPool)

use std::net::{TcpListener, TcpStream};
use std::io::prelude::*;
use std::sync::{Arc, Mutex};

use byteorder::{ByteOrder, LittleEndian};

use tflitec::interpreter::{Interpreter, Options};

use remote_server::ThreadPool; // IMPORT THREADPOOL CAPABILITY

const BUFFER1_SIZE: usize = 110592; // BUFFER1 : READING IN
const BUFFER2_SIZE: usize = 204; // BUFFER2 : WRITING OUT
const BUFFER3_SIZE: usize = 4; // BUFFER3 : STORING A SINGLE FLOAT

fn main() {
    // POSSIBLE CODES
    //      between two VMs     :      [::]:8000
    //      within the same VM  : 127.0.0.1:8000

    let listener: TcpListener = TcpListener::bind("127.0.0.1:8000").expect("Set up server [FAILED]"); // SET UP SERVER
    let pool = ThreadPool::new(4); // CREATE THREADPOOL

    // LOADING THE MODEL/INTERPRETER
	let path = format!("resource/lite-model_movenet_singlepose_lightning_tflite_int8_4.tflite");
	
    let interpreter = Interpreter::with_model_path(&path, Some(Options::default())).expect("Load model [FAILED]");
	interpreter.allocate_tensors().expect("Allocate tensors [FAILED]");
    let interpreter = Arc::new(Mutex::new(interpreter)); // CREATE A MUTEXED ATOMIC REFERENCE TO THE INTERPRETER

    // ACCEPT CONNECTIONS USING TCPSTREAM
    for stream in listener.incoming() {
        let stream: TcpStream = stream.expect("Finding connection [FAILED]");
        let interpreter = Arc::clone(&interpreter);

        pool.execute(move || {
            handle_connection(stream, interpreter);
        });
    }
}

fn handle_connection(mut stream: TcpStream, interpreter: Arc<Mutex<Interpreter>>) {
    // READ INFORMATION IN FROM THE STREAM
    let mut buffer: [u8; BUFFER1_SIZE] = [0; BUFFER1_SIZE]; // CREATE BUFFER
    stream.read(&mut buffer).expect("Reading stream [FAILED]"); // READ IN THE DATA
    
    let vec_1d: Vec<u8> = buffer.to_vec();

    // SET THE INPUT TO THE INTERPRETER
    let interpreter = interpreter.lock().expect("Unlocking interpreter [FAILED]");
    interpreter.copy(&vec_1d[..], 0).expect("Copying data into interpreter [FAILED]");
    
    interpreter.invoke().expect("Invoke [FAILED]"); // RUN THE INTERPRETER

    // GET THE OUTPUT FROM THE INTERPRETER
    let output_tensor = interpreter.output(0).expect(" [FAILED]");
    let output_tensor = output_tensor.data::<f32>();

    // CONVERT TO OUTPUT DATA TO BYTES
    let mut buffer2: [u8; BUFFER2_SIZE] = [0; BUFFER2_SIZE];
    let mut buffer3: [u8; BUFFER3_SIZE] = [0; BUFFER3_SIZE];
    
    for i in 0..51 {
        LittleEndian::write_f32(&mut buffer3, output_tensor[i]);
        for j in 0..4 {
            buffer2[i * 4 + j] = buffer3[j];
        }
    }

    // WRITE BACK TO THE CALLER
    stream.write_all(&buffer2[..]).expect("Writing back to caller [FAILED]");
    stream.flush().expect("Flushing the stream [FAILED]");
}
