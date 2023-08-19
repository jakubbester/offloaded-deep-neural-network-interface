/// Setting up the server to receive connections/stream data from user (using ThreadPool)

use std::net::{TcpListener, TcpStream};
use std::io::{prelude::*, Cursor};
use std::sync::{Arc, Mutex};

use byteorder::{ByteOrder, LittleEndian};

use tflitec::interpreter::{Interpreter, Options};

use remote_server::ThreadPool; // IMPORT THREADPOOL CAPABILITY

use image::imageops::Nearest;
use image;
use image::io::Reader;

const V4L2_PIX_WIDTH: u32 = 800; // WIDTH OF IMAGE
const V4L2_PIX_HEIGHT: u32 = 448; // HEIGHT OF IMAGE

const BUFFER1_SIZE: usize = V4L2_PIX_WIDTH as usize * V4L2_PIX_HEIGHT as usize * 4; // BUFFER1 : READING IN
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

// HELPER FUNCTIONS

fn handle_connection(mut stream: TcpStream, interpreter: Arc<Mutex<Interpreter>>) {
    // READ INFORMATION IN FROM THE STREAM
    let mut buffer: [u8; BUFFER1_SIZE] = [0; BUFFER1_SIZE]; // CREATE BUFFER
    stream.read(&mut buffer).expect("Reading stream [FAILED]"); // READ IN THE DATA

    // READ IN THE IMAGE, CONVERT TO RGB, AND GET RAW DATA
    let image = Reader::new(Cursor::new(&buffer)).with_guessed_format().unwrap().decode().unwrap();
    let image = image.resize_exact(192, 192, Nearest);
    let image = image.to_rgb8();
    let image = image.into_raw();

    // SET THE INPUT TO THE INTERPRETER
    let interpreter = interpreter.lock().expect("Unlocking interpreter [FAILED]");
    interpreter.copy(&image, 0).expect("Copying data into interpreter [FAILED]");
    
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

/// Implemented for when dealing with YUV422
fn _buff_yuv422to_rgb888(yuv422: &[u8]) -> Vec<u8> {
    let mut rgb888 = Vec::new(); // CREATE RESULTING VECTOR
    
    for i in 0..yuv422.len() / 4 {
        // DECOMPOSING THE YUV422 ENTRIES
        let (y1, u, y2, v) = (
            yuv422[i * 4] as f64,
            yuv422[i * 4 + 1] as f64,
            yuv422[i * 4 + 2] as f64,
            yuv422[i * 4 + 3] as f64,
        );

        // CALCULATING THE RGB VALUES
        let (rgb1, rgb2) = (
            _yuv422to_rgb888(y1, u, v), 
            _yuv422to_rgb888(y2, u, v),
        );

        // PUSHING THE RESULTS
        rgb888.extend(rgb1);
        rgb888.extend(rgb2);
    }

    return rgb888; // RETURN RESULTING VECTOR
}

fn _yuv422to_rgb888(y: f64, u: f64, v: f64) -> [u8; 3] {
    return [
        (y + 1.4075 * (v - 128.0)) as u8,
        (y - 0.3455 * (u - 128.0) - (0.7169 * (v - 128.0))) as u8,
        (y + 1.7790 * (u - 128.0)) as u8,
    ]; // CONVERT AND RETURN AN ARRAY OF [R, G, B] VALUES
}
