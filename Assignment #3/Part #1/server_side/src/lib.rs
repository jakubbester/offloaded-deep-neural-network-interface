use std::{fs::File, os::unix::prelude::AsRawFd, ptr::null_mut};
use std::net::TcpStream; // NETWORKING
use std::io::{prelude::*, Cursor}; // READ/WRITE CAPABILITY
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::{thread, time};
use std::sync::{Arc, Mutex};

use libc::{mmap, PROT_READ, PROT_WRITE, MAP_SHARED, MAP_FAILED}; // MMAP FUNCTIONALITY
use nix::{ioctl_read, ioctl_write_int, ioctl_readwrite}; // IOCTL SYSTEM CALLS
use byteorder::{ByteOrder, LittleEndian};

use tflitec::interpreter::{Interpreter};

use image::imageops::Nearest;
use image;
use image::io::Reader;

use opencv::core::CV_8UC3;
use opencv::{
    prelude::*,
    highgui::*
}; // CAMERA TOOLS

mod utils; // UTILITY FUNCTIONS
use utils::*;

// BUFFER SIZES

const BUFFER1_SIZE: usize = 589824;
const BUFFER2_SIZE: usize = 4;
const BUFFER3_SIZE: usize = 204;
const BUFFER4_SIZE: usize = 51;

// STATIC VARIABLES

static ANNOTATE: AtomicBool = AtomicBool::new(false);
static KEY: AtomicI32 = AtomicI32::new(97);
static DELAY: AtomicU64 = AtomicU64::new(40);

// CAPABILITY CONSTANTS

// #define VIDIOC_QUERYCAP          _IOR('V',  0, struct v4l2_capability)

const VIDIOC_QUERYCAP_MAGIC: u8 = 'V' as u8;
const VIDIOC_QUERYCAP_TYPE_MODE: u8 = 0;

const V4L2_CAP_VIDIO_CAPTURE: u32 = 0x00000001;
const V4L2_CAP_STREAMING: u32 = 0x04000000;

// FORMAT CONSTANTS/MACROS

// #define VIDIOC_G_FMT             _IOWR('V',  4, struct v4l2_format)
// #define VIDIOC_S_FMT             _IOWR('V',  5, struct v4l2_format)

const VIDIOC_G_FMT_MAGIC: u8 = 'V' as u8;
const VIDIOC_G_FMT_TYPE_MODE: u8 = 4;
const VIDIOC_S_FMT_MAGIC: u8 = 'V' as u8;
const VIDIOC_S_FMT_TYPE_MODE: u8 = 5;

// #define v4l2_fourcc(a, b, c, d) \
//      ((__u32)(a) | ((__u32)(b) << 8) | ((__u32)(c) << 16) | ((__u32)(d) << 24))

macro_rules! v4l2_fourcc {
    ($a:expr, $b:expr, $c:expr, $d:expr, $typ:ty) => {
        ($a as $typ) | ($b as $typ) << 8 | ($c as $typ) << 16 | ($d as $typ) << 24
    }
}

const V4L2_PIX_FMT_MJPG: u32 = v4l2_fourcc!(b'M', b'J', b'P', b'G', u32);

// CUSTOM FORMAT SIZING CONSTANTS

const V4L2_PIX_WIDTH: u32 = 800;
const V4L2_PIX_HEIGHT: u32 = 448;

// FORMAT TO USE
//      size : 800x448
//      rate : 30.000 fps

const V4L2_BUF_TYPE_VIDEO_CAPTURE: u32 = 1;

// REQUESTBUFFERS CONSTANTS

// #define VIDIOC_REQBUFS           _IOWR('V',  8, struct v4l2_requestbuffers)

const VIDIOC_REQBUFS_MAGIC: u8 = 'V' as u8;
const VIDIOC_REQBUFS_TYPE_MODE: u8 = 8;

const V4L2_MEMORY_MMAP: u32 = 1;

// BUFFER CONSTANTS

// #define VIDIOC_QUERYBUF          _IOWR('V',  9, struct v4l2_buffer)

const VIDIOC_QUERYBUF_MAGIC: u8 = 'V' as u8;
const VIDIOC_QUERYBUF_TYPE_MODE: u8 = 9;

// QUEUE BUFFER CONSTANTS

// #define VIDIOC_QBUF              _IOWR('V', 15, struct v4l2_buffer)

const VIDIOC_QBUF_MAGIC: u8 = 'V' as u8;
const VIDIOC_QBUF_TYPE_MODE: u8 = 15;

// DEQUEUE BUFFER CONSTANTS

// #define VIDIOC_DQBUF             _IOWR('V', 17, struct v4l2_buffer)

const VIDIOC_DQBUF_MAGIC: u8 = 'V' as u8;
const VIDIOC_DQBUF_TYPE_MODE: u8 = 17;

// STREAM ON CONSTANTS

// #define VIDIOC_STREAMON          _IOW('V', 18, int)

const VIDIOC_STREAMON_MAGIC: u8 = 'V' as u8;
const VIDIOC_STREAMON_TYPE_MODE: u8 = 18;

// STREAM OFF CONSTANTS

// #define VIDIOC_STREAMOFF         _IOW('V', 19, int)

const VIDIOC_STREAMOFF_MAGIC: u8 = 'V' as u8;
const VIDIOC_STREAMOFF_TYPE_MODE: u8 = 19;

// IOCTL STRUCTS

#[repr(C)]
#[derive(Default, Debug)]
pub struct v4l2_capability {
    pub driver: [u8; 16],
    pub card: [u8; 32],
    pub bus_info: [u8; 32],
    pub version: u32,
    pub capabilities: u32,
    pub device_caps: u32,
    pub reserved: [u32; 3]
}

#[repr(C)]
#[derive(Debug)]
pub struct v4l2_format {
    pub r#type: u32,
    pub align1: u32,
    pub width: u32,
    pub height: u32,
    pub pixelformat: u32,
    pub others1: [u8; 208 - 5 * 4]
}

impl Default for v4l2_format {
    fn default() -> v4l2_format {
        v4l2_format {
            r#type: 0,
            align1: 0,
            width: 0,
            height: 0,
            pixelformat: 0,
            others1: [0; 208 - 5 * 4]
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct v4l2_requestbuffers {
    pub count: u32,
    pub r#type: u32,
    pub memory: u32,
    pub reserved: [u32; 2]
}

impl Default for v4l2_requestbuffers {
    fn default() -> v4l2_requestbuffers {
        v4l2_requestbuffers {
            count: 0,
            r#type: 0,
            memory: 0,
            reserved: [0, 0]
        }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct v4l2_buffer {
    pub index: u32,
    pub r#type: u32,
    pub others1: [u32; 13],
    pub memory: u32,
    pub offset: u32,
    pub others2: u32,
    pub length: u32,
    pub others3: [u32; 3]
}

// STRING FORMATTING CONSTANTS
static OK: &'static str = "[OK]";
static FAIL: &'static str = "[FAILED]";
static OFFSET: usize = 35;

// PRIVATE HELPER FUNCTIONS

fn connect() -> TcpStream {
	// POSSIBLE CODES
	//      between two VMs     :   <ipv4> :8000 of remote server
	//      within the same VM  : 127.0.0.1:8000

    return TcpStream::connect("127.0.0.1:8000").expect("Connection [FAILED]"); // 192.168.25.130
}

// pfcode : print formatted code
fn pfcode(message: &str, code: &str) {
    println!("{} {} {}", message, format!("{: ^width$}", "", width = OFFSET - message.len()), code);
}

// PUBLIC/PUBLISHED FUNCTIONS

pub fn display(interpreter: Arc<Mutex<Interpreter>>) {
	println!("SETTING UP CAMERA ...\n");

	// OPEN DEVICE FILE /dev/video0 AND GET FILE DESCRIPTOR

	let file = File::options().write(true).read(true).open("/dev/video0").unwrap();
	let fd = file.as_raw_fd();

	// GATHER INFORMATION ABOUT VIDEO FILE

	let mut capability: v4l2_capability = Default::default();
	ioctl_read!(vidioc_querycap, VIDIOC_QUERYCAP_MAGIC, VIDIOC_QUERYCAP_TYPE_MODE, v4l2_capability);

	match unsafe { vidioc_querycap(fd, &mut capability as *mut v4l2_capability) } {
		Ok(_) => {
			pfcode("Get Information", OK);

			if &capability.capabilities & V4L2_CAP_VIDIO_CAPTURE > 0 {
				pfcode("Single-Planar Video Capture", OK);
			} else {
				pfcode("Single-Planar Video Capture", FAIL);
			}

			if &capability.capabilities & V4L2_CAP_STREAMING > 0 {
				pfcode("Streaming", OK);
			} else {
				pfcode("Streaming", FAIL);
			}
		}, Err(e) => {
			pfcode("Get Information", &(FAIL.to_owned() + &format!(": {:?}", e).to_owned()));
		}
	}

	// GET/SET THE FORMAT OF THE DEVICE

	let mut format: v4l2_format = Default::default();
	format.r#type = V4L2_BUF_TYPE_VIDEO_CAPTURE;

	ioctl_readwrite!(vidioc_g_fmt, VIDIOC_G_FMT_MAGIC, VIDIOC_G_FMT_TYPE_MODE, v4l2_format);
	match unsafe { vidioc_g_fmt(fd, &mut format as *mut v4l2_format) } {
		Ok(_) => {
			pfcode("Get Formatting", OK);
		} Err(e) => {
			pfcode("Get Formatting", &(FAIL.to_owned() + &format!(": {:?}", e).to_owned()));
		}
	}

	format.width = V4L2_PIX_WIDTH;
	format.height = V4L2_PIX_HEIGHT;
	format.pixelformat = V4L2_PIX_FMT_MJPG;

	ioctl_readwrite!(vidioc_s_fmt, VIDIOC_S_FMT_MAGIC, VIDIOC_S_FMT_TYPE_MODE, v4l2_format);
	match unsafe { vidioc_s_fmt(fd, &mut format as *mut v4l2_format) } {
		Ok(_) => {
			pfcode("Set Formatting", OK);
		}, Err(e) => {
			pfcode("Set Formatting", &(FAIL.to_owned() + &format!(": {:?}", e).to_owned()));
		}
	}

	// PRINT INFORMATION TO ENSURE SET CORRECTLY
	// println!("width: {}", format.width);
	// println!("height: {}", format.height);
	// println!("pixelformat: {}", format.pixelformat);

	// REQUEST BUFFERS FROM THE DEVICE

	let mut requestbuffers: v4l2_requestbuffers = Default::default();
	requestbuffers.count = 1;
	requestbuffers.r#type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
	requestbuffers.memory = V4L2_MEMORY_MMAP;

	ioctl_readwrite!(vidioc_reqbufs, VIDIOC_REQBUFS_MAGIC, VIDIOC_REQBUFS_TYPE_MODE, v4l2_requestbuffers);
	match unsafe { vidioc_reqbufs(fd, &mut requestbuffers as *mut v4l2_requestbuffers) } {
		Ok(_) => {
			pfcode("Request Buffers", OK);
		}, Err(e) => {
			pfcode("Request Buffers", &(FAIL.to_owned() + &format!(": {:?}", e).to_owned()));
		}
	}

	// QUERYING BUFFERS FROM DEVICE

	let mut buffer: v4l2_buffer = Default::default();
	buffer.index = 0;
	buffer.r#type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
	buffer.memory = V4L2_MEMORY_MMAP;

	ioctl_readwrite!(vidioc_querybuf, VIDIOC_QUERYBUF_MAGIC, VIDIOC_QUERYBUF_TYPE_MODE, v4l2_buffer);
	match unsafe { vidioc_querybuf(fd, &mut buffer as *mut v4l2_buffer) } {
		Ok(_) => {
			pfcode("Query Buffers", OK);
		} Err(e) => {
			pfcode("Query Buffers", &(FAIL.to_owned() + &format!(": {:?}", e).to_owned()));
		}
	}

	// MEMORY MAPPING FOR THE BUFFERS

	let data;
	unsafe {
		data = mmap(
			/* addr */      null_mut(),
			/* len */       buffer.length.try_into().unwrap(),
			/* prot */      PROT_READ | PROT_WRITE,
			/* Make the mapping *public* so that it is written into the file */
			/* flags */     MAP_SHARED,
			/* fd */        fd,
			/* offset */    buffer.offset.try_into().unwrap()
		);

		if data == MAP_FAILED {
			panic!("Can't create mmapped file!");
		} else {
			pfcode("MMAP Created", OK);
		}
	}

	// QUEUE BUFFER

	buffer.index = 0;
	buffer.r#type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
	buffer.memory = V4L2_MEMORY_MMAP;
	ioctl_readwrite!(vidioc_qbuf, VIDIOC_QBUF_MAGIC, VIDIOC_QBUF_TYPE_MODE, v4l2_buffer);
	match unsafe { vidioc_qbuf(fd, &mut buffer as *mut v4l2_buffer) } {
		Ok(_) => {
			pfcode("Queueing Buffer", OK);
		} Err(e) => {
			pfcode("Queueing Buffer", &(FAIL.to_owned() + &format!(": {:?}", e).to_owned()));
		}
	}

	// ACTIVATE STREAMING

	buffer.r#type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
	ioctl_write_int!(vidioc_streamon, VIDIOC_STREAMON_MAGIC, VIDIOC_STREAMON_TYPE_MODE);
	match unsafe { vidioc_streamon(fd, std::mem::transmute::<&u32, u64>(&buffer.r#type)) } {
		Ok(_) => {
			pfcode("Turning Stream On", OK);
		} Err(e) => {
			pfcode("Turning Stream On", &(FAIL.to_owned() + &format!(": {:?}", e).to_owned()));
		}
	}

	let length = buffer.length.to_string();
	println!("");
	pfcode("Buffer Length", &length);

	// RUNNING LOOPS
	//      one which runs for as long as you want
	//      and capture frames (shoots the video)
	//      and one that iterates over the buffers (optional)

	if ANNOTATE.load(Ordering::Relaxed) == true {
		println!("\nDISPLAYING VIDEO FEED w/ ANNOTATION\n");
	} else {
		println!("\nDISPLAYING VIDEO FEED w/o ANNOTATION\n");
	}

	println!("PRESS [SET KEY] or ^C TO EXIT THE FEED\n");
	loop {

		// DEQUEUE BUFFER

		ioctl_readwrite!(vidioc_dqbuf, VIDIOC_DQBUF_MAGIC, VIDIOC_DQBUF_TYPE_MODE, v4l2_buffer);
		match unsafe { vidioc_dqbuf(fd, &mut buffer as *mut v4l2_buffer) } {
			Ok(_) => { } Err(e) => {
				panic!("Dequeueing Buffer [FAILED]: {}", e);
			}
		}

		// GET RAW DATA STORED IN MMAP

		let raw: &[u8];
		unsafe {
			raw = std::slice::from_raw_parts(
				data as *const u8,
				(V4L2_PIX_WIDTH * V4L2_PIX_HEIGHT * 4) as usize
			);
		}

		// CREATE EMPTY IMAGE MATRIX

		let mut image = Mat::zeros(
			V4L2_PIX_HEIGHT as i32, V4L2_PIX_WIDTH as i32, CV_8UC3
		).unwrap().to_mat().unwrap();

		if ANNOTATE.load(Ordering::Relaxed) == true {
			// READ IN THE IMAGE, CONVERT TO RGB, AND GET RAW DATA
			let figure = Reader::new(Cursor::new(&raw)).with_guessed_format().unwrap().decode().unwrap();
			let figure = figure.resize_exact(192, 192, Nearest);
			let figure = figure.to_rgb8();
			let figure = figure.into_raw();

			// RUN LOCAL COMPONENT OF MODEL
			let interpreter = interpreter.lock().expect("Unlocking interpreter [FAILED]");
			interpreter.copy(&figure, 0).expect("Copying data into interpreter [FAILED]");

			interpreter.invoke().expect("Invoke [FAILED]"); // RUN THE INTERPRETER

			// GET THE OUTPUT FROM THE INTERPRETER
			let output_tensor = interpreter.output(0).expect(" [FAILED]");
			let output_tensor = output_tensor.data::<f32>();

			// CONVERT OUTPUT DATA TO BYTES
			let mut buffer1: [u8; BUFFER1_SIZE] = [0; BUFFER1_SIZE];
			let mut buffer2: [u8; BUFFER2_SIZE] = [0; BUFFER2_SIZE];

			for i in 0..147456 {
				LittleEndian::write_f32(&mut buffer2, output_tensor[i]);
				for j in 0..4 {
					buffer1[i * 4 + j] = buffer2[j];
				}
			}

			// WRITE DATA TO THE STREAM
			let mut stream = connect();
			stream.write(&buffer1).expect("Write to stream [FAILED]");

			// ADD DELAY WHEN CONNECTION IS FURTHER AWAY (e.g. BETWEEN TWO VMs)
			thread::sleep(
				time::Duration::from_millis(DELAY.load(Ordering::Relaxed))
			); // rather arbitrary for now

			// READ DATA FROM THE STREAM
			let mut buffer3: [u8; BUFFER3_SIZE] = [0; BUFFER3_SIZE];

			stream.read(&mut buffer3).expect("Reading from stream [FAILED]");

			// CONVERT BACK TO FLOATING POINT
			let mut buffer4: [f32; BUFFER4_SIZE] = [0.0; BUFFER4_SIZE];
			for i in 0..BUFFER4_SIZE {
				buffer2 = buffer3[i * 4..i * 4 + 4].try_into().expect("Taking slice [FAILED]");
				buffer4[i] = LittleEndian::read_f32(&mut buffer2)
			}

			draw_keypoints(&mut image, &buffer4, 0.25);
		}

		// DISPLAY RESULT

		imshow("MoveNet", &image).expect("imshow [ERROR]");

		// QUEUE BUFFER

		ioctl_readwrite!(vidioc_qbuf, VIDIOC_QBUF_MAGIC, VIDIOC_QBUF_TYPE_MODE, v4l2_buffer);
		match unsafe { vidioc_qbuf(fd, &mut buffer as *mut v4l2_buffer) } {
			Ok(_) => { } Err(e) => {
				panic!("Queueing Buffer [FAILED]: {}", e);
			}
		}

		// CHECK FOR A KEYPRESS TO TERMINATE PROGRAM

		let key = wait_key(1).expect("Wait key [FAILED]");
		if key == KEY.load(Ordering::Relaxed) {
			break;
		}
	}

	// DEACTIVATE STREAMING

	ioctl_write_int!(vidioc_streamoff, VIDIOC_STREAMOFF_MAGIC, VIDIOC_STREAMOFF_TYPE_MODE);
	match unsafe { vidioc_streamoff(fd, std::mem::transmute::<&u32, u64>(&buffer.r#type)) } {
		Ok(_) => {
			pfcode("Turning Stream Off\n", OK);
		} Err(e) => {
			pfcode("Turning Stream Off\n", &(FAIL.to_owned() + &format!(": {:?}", e).to_owned()));
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
