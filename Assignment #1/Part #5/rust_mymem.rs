//! mymem module in Rust.

use kernel::prelude::*;
use kernel::sync::smutex::Mutex;

// DEFINE GLOBAL/CONSTANT VARIABLES
const MEM_SIZE:usize = 524288;

module! {
    type: MyMem,
    name: "mymem",
    author: "Jakub Bester",
    description: "MyMem memory module",
    license: "GPL",
}

// IMPLEMENTING THE SHARED BUFFER CODE

/// MyMem struct that is exported to be used externally
pub struct MyMem;

/// DEVICE that is mutexed and written/read from
pub static DEVICE: Mutex<Vec<u8>> = Mutex::new(Vec::new());

// IMPLEMENTING READ AND WRITE CAPABILITY
impl MyMem {
    /// Read from the virtual DEVICE to the reader's buffer.
    pub fn read(&mut self, outbuf: &mut [u8], offset: usize) -> usize {
        // INFORM USER THAT INFORMATION WAS READ
        // pr_info!("Information from the file was read");

        if offset + outbuf.len() > MEM_SIZE {
            pr_err!("Error reading from the storage container");
            return 0;
        }

        let device = DEVICE.lock();
        for i in offset..offset + outbuf.len() {
            outbuf[i] = device[i];
        }
    
        return outbuf.len();
    }
    
    /// Write from the writer's buffer to the virtual DEVICE
    pub fn write(&mut self, inbuf: &[u8], offset: usize) -> usize {
        // INFORM USER THAT INFORMATION WAS WRITTEN
        // pr_info!("Information was written to the file");

        if offset + inbuf.len() > MEM_SIZE {
            pr_err!("Error writing to the storage container");
            return 0;
        }

        let mut device = DEVICE.lock();
        for i in offset..offset + inbuf.len() {
            device[i] = inbuf[i];
        }
    
        return inbuf.len();
    }
}

// IMPLEMENTING THE OPERATIONS THAT DEFINE THE MODULE

impl kernel::Module for MyMem {
    fn init(_name: &'static CStr, _module: &'static ThisModule) -> Result<Self> {
        // OPENING UP THE MODULE
        pr_info!("Loading in the MyMem module!");

        let mut device = DEVICE.lock();
        for _ in 0..MEM_SIZE {
            device.try_push(0).unwrap();
        }

        Ok(MyMem {})
    }
}

impl Drop for MyMem {
    fn drop(&mut self) {
        // CLOSING OUT THE MODULE
        // pr_info!("Closing the MyMem module!");
    }
}
