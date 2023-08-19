//! mymem module in Rust.

use kernel::io_buffer::*;
use kernel::prelude::*;
use kernel::{file, miscdev};
use kernel::sync::{Mutex, Ref, RefBorrow, UniqueRef};

// DEFINE GLOBAL/CONSTANT VARIABLES
const MEM_SIZE_U64 : u64 = 512 * 1024;
const MEM_SIZE_USIZE : usize = MEM_SIZE_U64 as usize;

module! {
    type: MyMem,
    name: "mymem",
    author: "Jakub Bester",
    description: "MyMem memory module",
    license: "GPL",
}

// IMPLEMENTING THE SHARED STATE CODE

struct SharedStateInner {
    mem_buffer: [u8 ; MEM_SIZE_USIZE],
    offset: u64,
}

struct SharedState {
    inner: Mutex<SharedStateInner>,
}

impl SharedState {
    fn try_new() -> Result<Ref<Self>> {
        let mut state = Pin::from(UniqueRef::try_new(Self {
            inner: unsafe {
                Mutex::new(SharedStateInner {
                    mem_buffer: [0; MEM_SIZE_USIZE],
                    offset: 0,
                })},
        })?);

        let pinned = unsafe { state.as_mut().map_unchecked_mut(|s| &mut s.inner) };
        kernel::mutex_init!(pinned, "SharedState::inner");

        Ok(state.into())
    }
}

// IMPLEMENTING THE OPERATIONS THAT DEFINE THE DEVICE FILE

#[vtable]
impl file::Operations for MyMem {
    type Data = Ref<SharedState>;
    type OpenData = Ref<SharedState>;

    fn open(shared: &Ref<SharedState>, _file: &file::File) -> Result<Self::Data> {
        pr_info!("File was opened\n");
        Ok(shared.clone())
    }

    fn read(shared: RefBorrow<'_, SharedState>, _file: &file::File, writer: &mut impl IoBufferWriter, _offset: u64,) -> Result<usize> {
        let mut inner = shared.inner.lock();
        let length = writer.len();
        let offset_usize = inner.offset as usize;
        let length_u64 = length as u64;

        if offset_usize + length > MEM_SIZE_USIZE {
            return Err(EINVAL);
        }

        // PUT MESSAGE TO THE BUFFER
        writer.write_slice(&inner.mem_buffer[offset_usize..offset_usize + length])?;
        inner.offset += length_u64;

        // INFORM USER THAT INFORMATION WAS READ
        pr_info!("Information from the file was read");

        Ok(length)
    }

    fn write(shared: RefBorrow<'_, SharedState>, _file: &file::File, reader: &mut impl IoBufferReader, _offset: u64,) -> Result<usize> {
        let mut inner = shared.inner.lock();
        let length = reader.len();
        let length_u64 = length as u64;
        let offset_usize = inner.offset as usize;
        
        if offset_usize + length > MEM_SIZE_USIZE {
            return Err(EINVAL);
        }

        // PUT THE MESSAGE INTO THE FILE
        reader.read_slice(&mut inner.mem_buffer[offset_usize..offset_usize + length])?;
        inner.offset += length_u64;

        // INFORM USER THAT INFORMATION WAS WRITTEN
        pr_info!("Information was written to the file");

        Ok(length)
    }

    fn seek(shared: RefBorrow<'_, SharedState>, _file: &file::File, _offset: file::SeekFrom,) -> Result<u64> {
        let mut inner = shared.inner.lock();
        match _offset {
            file::SeekFrom::Start(val) => {
                inner.offset = val;
                pr_info!("Seek was set to start");
            }
            file::SeekFrom::Current(val) => {
                inner.offset = inner.offset.wrapping_add(val as u64);
                pr_info!("Seek was set from current position");
            }
            file::SeekFrom::End(val) => {
                inner.offset = MEM_SIZE_U64.wrapping_add(val as u64);
                pr_info!("Seek was set from the ending position");
            }
        }

        if inner.offset > MEM_SIZE_U64 {
            return Err(EINVAL);
        }

        // TELLING THE USER THAT THE FILE POINTER CHANGED
        pr_info!("Location of the file pointer changed!");

        Ok(inner.offset)
    }

    fn release(_shared: Ref<SharedState>, _file: &file::File) {
        pr_info!("File was succesfully closed!");
    }
}

// IMPLEMENTING THE OPERATIONS THAT DEFINE THE MODULE

struct MyMem {
    _dev: Pin<Box<miscdev::Registration<MyMem>>>,
}

impl kernel::Module for MyMem {
    fn init(_name: &'static CStr, _module: &'static ThisModule) -> Result<Self> {
        pr_info!("Loading in the MyMem module!");

        // INITIALIZE THE STORAGE STRUCVT FOR HOLDING INFORMATION
        let state = SharedState::try_new()?;

        // REGISTER THE DEVICE FILE
        let reg = miscdev::Registration::new_pinned(fmt!("mymem"), state)?;

        Ok(MyMem {_dev: reg})
    }
}

impl Drop for MyMem {
    fn drop(&mut self) {
        // CLOSING OUT THE MODULE
        pr_info!("Rust miscellaneous device sample (exit)\n");
    }
}
