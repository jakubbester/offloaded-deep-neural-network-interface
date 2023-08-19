//! RustCamera module in Rust.

use kernel::prelude::*;
use kernel::sync::{smutex::Mutex, Ref, RefBorrow};
use kernel::bindings;
use kernel::net::{TcpStream};
use kernel::file::{File, Operations, IoctlHandler};
use kernel::{miscdev, Module};
use kernel::io_buffer::{IoBufferReader, IoBufferWriter};
use kernel::c_str;

// DEFINE GLOBAL/CONSTANT VARIABLES
const MEM_SIZE:usize = 524288;

const O_RDWR: i32 = 0x00000002;
const NAME_OF_FILE: &CStr = c_str!("/dev/video0");

module! {
    type: RustCamera,
    name: "rust_camera",
    author: "Jakub Bester",
    description: "A simple module that reads camera input.",
    license: "GPL",
}

// IMPLEMENTING PRIVATE HELPER FUNCTIONS
// fn connect() -> TcpStream {
//     // POSSIBLE CODES
// 	//      between two VMs     :   <ipv4> :8000 of remote server
// 	//      within the same VM  : 127.0.0.1:8000

//     return TcpStream::connect("127.0.0.1:8000").expect("Connection [FAILED]"); // 192.168.25.130:8000
// }

// IMPLEMENTING THE SHARED BUFFER CODE

/// RustCamera struct that is exported to be used externally
struct RustCamera {
    _dev: Pin<Box<miscdev::Registration<RustCamera>>>,
}

/// DEVICE that is mutexed and written/read from
struct Device {
    number: usize,
    contents: Mutex<Vec<u8>>,
}

// IMPLEMENTING READ AND WRITE CAPABILITY
#[vtable]
impl Operations for RustCamera {
    // The data that is passed into the open method
    type OpenData = Ref<Device>;
    // The data that is returned by running an open method
    type Data = Ref<Device>;

    fn open(
        context: &Ref<Device>,
        _file: &File
    ) -> Result<Ref<Device>> {
        // INFORM USER THAT INFORMATION WAS READ
        pr_info!("File for device {} was opened\n", context.number);
        Ok(context.clone())
    }

    // Read the data contents and write them into the buffer provided
    fn read(
        data: RefBorrow<'_, Device>,
        _file: &File,
        writer: &mut impl IoBufferWriter,
        offset: u64,
    ) -> Result<usize> {
        pr_info!("File for device {} was read\n", data.number);
        let offset = offset.try_into().unwrap();
        let vec = data.contents.lock();
        let len = core::cmp::min(writer.len(), vec.len().saturating_sub(offset));
        writer.write_slice(&vec[offset..][..len]).unwrap();
        Ok(len)
    }

    // Read from the buffer and write the data in the contents after locking the mutex
    fn write(
        data: RefBorrow<'_, Device>,
        _file: &File,
        reader: &mut impl IoBufferReader,
        _offset: u64,
    ) -> Result<usize> {
        pr_info!("File for device {} was written\n", data.number);
        let copy = reader.read_all().unwrap();
        let len = copy.len();
        *data.contents.lock() = copy;
        Ok(len)
    }
}

// IMPLEMENTING THE OPERATIONS THAT DEFINE THE MODULE

impl Module for RustCamera {
    fn init(_name: &'static CStr, _module: &'static ThisModule) -> Result<Self> {
        // OPENING UP THE MODULE
        pr_info!("Loading in the RustCamera module!");

        let mut buf:[u8; 8] = [0; 8];

        // OPENING THE DEVICE FILE /dev/video0
        let file = unsafe { kernel::bindings::filp_open(NAME_OF_FILE.as_char_ptr(), O_RDWR, 0) };
        // pr_info!("file name: {:?}", core::str::from_utf8(&(*(*file).f_path.dentry).d_iname)); // test if correct file opened

        let dev = Ref::try_new(Device { number: 1, contents: Mutex::new(Vec::new()) }).unwrap();
        let reg = miscdev::Registration::new_pinned(fmt!("RustCamera"), dev).unwrap();
        Ok(Self { _dev: reg })
    }
}

impl Drop for RustCamera {
    fn drop(&mut self) {
        // CLOSING OUT THE MODULE
        pr_info!("Closing the RustCamera module!");
    }
}
