use std::net::TcpStream; // NETWORKING
use std::io::prelude::*; // READ/WRITE CAPABILITY
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};

use opencv::core::{flip, Vec3b};
use opencv::videoio::*; // CAMERA TOOLS
use opencv::{
    prelude::*,
    videoio,
    highgui::*,
};

mod utils; // UTILITY FUNCTIONS
use utils::*;

use byteorder::{ByteOrder, LittleEndian};

const RESIZE: i32 = 192;
const BUFFER1_SIZE: usize = 204;
const BUFFER2_SIZE: usize = 4;
const BUFFER3_SIZE: usize = 51;

static ANNOTATE: AtomicBool = AtomicBool::new(false);
static KEY: AtomicI32 = AtomicI32::new(97);
static DELAY: AtomicU64 = AtomicU64::new(40);

use std::{thread, time};

// PRIVATE HELPER FUNCTIONS

fn connect() -> TcpStream {
	// POSSIBLE CODES
	//      between two VMs     :   <ipv4> :8000 of remote server
	//      within the same VM  : 127.0.0.1:8000

    return TcpStream::connect("192.168.25.130:8000").expect("Connection [FAILED]");
}

// PUBLIC/PUBLISHED FUNCTIONS

pub fn display() {
	println!("SETTING UP CAMERA ...\n");
	let mut camera = videoio::VideoCapture::new(0, videoio::CAP_ANY).expect("Setting up camera [FAILED]");
	videoio::VideoCapture::is_opened(&camera).expect("Open camera [FAILED]");
	camera.set(CAP_PROP_FPS, 30.0).expect("Set camera FPS [FAILED]");

	if ANNOTATE.load(Ordering::Relaxed) == true {
		println!("\nDISPLAYING VIDEO FEED w/ ANNOTATION\n");
	} else {
		println!("\nDISPLAYING VIDEO FEED w/o ANNOTATION\n");
	}

	println!("PRESS [SET KEY] or ^C TO EXIT THE FEED\n");
    loop {
        let mut frame = Mat::default();
		camera.read(&mut frame).expect("VideoCapture: read [FAILED]");

		if frame.size().expect("Frame size [FAILED]").width > 0 {
			let mut flipped = Mat::default();
			flip(&frame, &mut flipped, 1).expect("Flip [FAILED]"); // FLIP THE IMAGE HORIZONTALLY

			let resized_img = resize_with_padding(&flipped, [RESIZE, RESIZE]); // RESIZE IMAGE

			if ANNOTATE.load(Ordering::Relaxed) == true {
				// PERFORM VECTOR TRANSFORMATIONS
				let vec_2d: Vec<Vec<Vec3b>> = resized_img.to_vec_2d().expect("Converting to vector [FAILED]");
				let vec_1d: Vec<u8> = vec_2d.iter().flat_map(|v| v.iter().flat_map(|w| w.as_slice())).cloned().collect();

				// WRITE DATA TO THE STREAM
				let mut stream = connect();
				stream.write(&vec_1d[..]).expect("Write to stream [FAILED]");

				// ADD DELAY WHEN CONNECTION IS FURTHER AWAY (e.g. BETWEEN TWO VMs)
				thread::sleep(time::Duration::from_millis(DELAY.load(Ordering::Relaxed))); // rather arbitrary for now

				// READ DATA FROM THE STREAM
				let mut buffer1: [u8; BUFFER1_SIZE] = [0; BUFFER1_SIZE];
				let mut _buffer2: [u8; BUFFER2_SIZE] = [0; BUFFER2_SIZE];

				stream.read(&mut buffer1).expect("Reading from stream [FAILED]");

				// CONVERT BACK TO FLOATING POINT
				let mut buffer3: [f32; BUFFER3_SIZE] = [0.0; BUFFER3_SIZE];
				for i in 0..BUFFER3_SIZE {
					_buffer2 = buffer1[i * 4..i * 4 + 4].try_into().expect("Taking slice [FAILED]");
					buffer3[i] = LittleEndian::read_f32(&mut _buffer2)
				}

				draw_keypoints(&mut flipped, &buffer3, 0.25);
			}

			imshow("MoveNet", &flipped).expect("imshow [ERROR]"); // DISPLAY RESULT
		}

		// CHECK FOR A KEYPRESS TO TERMIANTE PROGRAM
		let key = wait_key(1).expect("Wait key [FAILED]");
		if key == KEY.load(Ordering::Relaxed) {
			break;
		}
	}
}

// ANNOTATE FUNCTION

pub fn annotate(annotate: bool) {
	println!("\nSETTING ANNOTATION TO TRUE\n");
	ANNOTATE.store(annotate, Ordering::Relaxed);
}

pub fn terminate(key: i32) {
	println!("SETTING TERMINATING KEY TO {}\n", key);
	KEY.store(key, Ordering::Relaxed);
}

pub fn delay(delay: u64) {
	println!("SETTING THE READ DELAY TO {} MILLISECONDS\n", delay);
	DELAY.store(delay, Ordering::Relaxed);
}
