// SPDX-License-Identifier: GPL-2.0

//! Access to the camera.

use kernel::prelude::*;
use kernel::str::CString;
use kernel::{
    file::{self, File},
    io_buffer::{IoBufferReader, IoBufferWriter},
    miscdev,
    sync::{Ref, RefBorrow, UniqueRef},
    sync::smutex::Mutex,
};

use kernel::bindings::{socket, sock_create, vfs_ioctl};
use core::ffi::c_int;

module! {
    type: RustCamera,
    name: "rust_camera",
    author: "Daniel Luick",
    description: "A simple module that reads camera input.",
    license: "GPL",
}

const W: usize = 400;
const H: usize = 712;

const PAGE_SIZE: usize = 4096;

// 4 addresses (vaddr+pfn pairs) are passed from userspace:
// - 2 v4l2_buffer structs (2nd is unused)
// - 2 mmap'd addresses since we use 2 buffers
const N_ADDRS: usize = 4;
const INPUT_SIZE: usize = N_ADDRS*2*8;

const BUF_SIZE: usize = (W*H)+((W*H)>>1);
const OUTPUT_SIZE: usize = 17*4*3;

/*******************************
 * v4l2 declarations & helpers *
 *******************************/

// const V4L2_BUF_TYPE_VIDEO_CAPTURE: usize = 1;
// const V4L2_MEMORY_MMAP: usize = 1;

fn xioctl(filp: *mut bindings::file, code: u32, vaddr: u64) -> Option<()> {
    loop {
        let result = unsafe { vfs_ioctl(filp, code, vaddr) };
        match result {
            0 => { return Some(()); }
            4 => {} // EINTR
            11 => {} // EAGAIN
            _ => {
                pr_info!("xioctl error, returned code {}\n", result);
                return None;
            }
        }
    }
}

// const FORMAT_PADDING: usize = 208 - 10*4;
// #[repr(C)]
// struct v4l2_format {
//     type_: u32,
//     space: u32,
//     width: u32,
//     height: u32,
//     pixelformat: u32,
//     field: u32,
//     bytesperline: u32,
//     sizeimage: u32,
//     colorspace: u32,
//     priv_: u32,
//     others: [u8; FORMAT_PADDING]
// }

#[repr(C)]
union anon_plane_m {
    mem_offset: u32,
    userptr: u64,
    fd: i32
}

#[repr(C)]
struct v4l2_plane {
    bytesused: u32,
    length: u32,
    m: anon_plane_m,
    data_offset: u32,
    reserved: [u32; 11]
}

#[repr(C)]
#[derive(Default)]
struct v4l2_requestbuffers {
    count: u32,
    type_: u32,
    memory: u32,
    capabilities: u32,
    flags: u8,
    reserved: [u8; 3]
}

#[repr(C)]
#[derive(Clone, Copy)]
union anon_buffer_m {
    offset: u32,
    userptr: u64,
    planes: *mut v4l2_plane,
    fd: i32
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct v4l2_timecode {
    type_: u32,
    flags: u32,
    frame: u8,
    seconds: u8,
    minutes: u8,
    hours: u8,
    userbits: [u8; 4]
}

// https://docs.rs/libc/0.2.137/libc/struct.timeval.html
// /usr/include/bits/types/struct_timeval.h
// assumes __USE_TIME_BITS64
#[repr(C)]
#[derive(Clone, Copy)]
struct timeval {
    tv_sec: i64,
    tv_usec: i64
}

#[repr(C)]
#[derive(Clone, Copy)]
struct v4l2_buffer {
    index: u32,
    type_: u32,
    bytesused: u32,
    flags: u32,
    field: u32,
    timestamp: timeval,
    timecode: v4l2_timecode,
    sequence: u32,

    // memory location
    memory: u32,
    m: anon_buffer_m,
    length: u32,
    reserved2: u32,
    anon_union: u32 // technically u32/s32
}

// https://www.kernel.org/doc/html/latest/userspace-api/ioctl/ioctl-decoding.html
const IORW: u32 = 0b11;
// const IOW: u32 = 0b01;
// const IOR: u32 = 0b10;
fn ioctl_num(ioty: u32, num: u32, size: u32) -> u32 {
    (ioty << 30) + (size << 16) + (('V' as u32) << 8) + num
}

/*******************
 * Networking code *
 *******************/

fn u8_array_of_u64(x: u64) -> [u8; 8] {
    let mut ans = [0; 8];

    for i in 0..8 {
        ans[i] = ((x >> (8*i)) % 256) as u8;
    }

    ans
}

fn u64_of_array(arr: &[u8]) -> u64 {
    let mut ans: u64 = 0;

    for i in 0..8 {
        ans += (arr[i] as u64) << (8*i);
    }

    ans
}

// https://rust-for-linux.github.io/docs/src/kernel/net.rs.html#335-358
fn sock_read(sock: *mut bindings::socket, buf: &mut [u8]) -> i64 {
    let mut msg = bindings::msghdr::default();
    let mut vec = bindings::kvec {
        iov_base: buf.as_mut_ptr().cast(),
        iov_len: buf.len(),
    };
    // SAFETY: The type invariant guarantees that the socket is valid, and `vec` was
    // initialised with the output buffer.
    let r = unsafe {
        bindings::kernel_recvmsg(
            sock,
            &mut msg,
            &mut vec,
            1,
            vec.iov_len,
            0 as _,
            /*if block { 0 } else { bindings::MSG_DONTWAIT } as _, */
            )
    };
    r as _
    /*
    if r < 0 {
        /* Err(Error::from_kernel_errno(r)) */
        Err(r)
    } else {
        Ok(r as _)
    }*/
}

// https://rust-for-linux.github.io/docs/src/kernel/net.rs.html#367-384
fn sock_write(sock: *mut bindings::socket, buf: &[u8]) -> i64 {
    let mut msg = bindings::msghdr {
        msg_flags: /*if block { 0 } else { bindings::MSG_DONTWAIT }*/ 0,
        ..bindings::msghdr::default()
    };
    let mut vec = bindings::kvec {
        iov_base: buf.as_ptr() as *mut u8 as _,
        iov_len: buf.len(),
    };
    // SAFETY: The type invariant guarantees that the socket is valid, and `vec` was
    // initialised with the input  buffer.
    let r = unsafe { bindings::kernel_sendmsg(sock, &mut msg, &mut vec, 1, vec.iov_len) };
    r as _
    /* if r < 0 {
        Err(Error::from_kernel_errno(r))
    } else {
        Ok(r as _)
    }*/
}

// TODO: this would contain info on the connection between frames, but I didn't get it to compile
// with rust so we create a new connection every frame for now & this is an empty struct.
struct Socket {
    // stream: TcpStream,
    // sock: socket,
}

impl Socket {
    fn new() -> Socket {
        Socket{}
    }

    // TODO: need to handle case where 1 read/write is not enough for all data.
    fn analyze(&mut self, data:&[u8]) -> [u8; OUTPUT_SIZE] {
        // let mut sock: socket = Default::default();
        let mut sock: *mut socket = core::ptr::null_mut();
        // https://elixir.bootlin.com/linux/latest/source/include/uapi/linux/in.h#L38
        // IPROTO_TCP = 6;
        // TODO: check return value
        unsafe { sock_create(bindings::PF_INET as c_int, bindings::sock_type_SOCK_STREAM as c_int, 6, &mut sock); }

        let mut sockaddr: bindings::sockaddr_in = Default::default();
        sockaddr.sin_family = bindings::AF_INET as _;
        sockaddr.sin_port = (8008 as u16).to_be();
        // let a: Ipv4Addr = Ipv4Addr::new(172, 28, 229, 170);
        sockaddr.sin_addr = bindings::in_addr { s_addr: u32::from_be_bytes([172, 28, 229, 170]).to_be() };

        // r = sock->ops->connect(sock, &mut sockaddr, sizeof(servaddr), bindings::O_RDWR);

        // Connect
        let a = unsafe { (*((*sock).ops)).connect.unwrap() };
        let x: *mut bindings::sockaddr_in = (&mut sockaddr) as *mut bindings::sockaddr_in;
        let y: *mut bindings::sockaddr = x as _;
        let r = unsafe {
            a(sock, y /*unsafe { (&mut sockaddr) as (&mut bindings::sockaddr) }*/ , core::mem::size_of::<bindings::sockaddr_in>() as i32 /*sizeof(servaddr)*/, bindings::O_RDWR as c_int)
        };
        pr_info!("connect return: {}\n", r);

        // Send length of data as u64, then send data.
        let len_array = u8_array_of_u64(data.len() as u64);
        sock_write(sock, &len_array); // TODO: might not write everything
        sock_write(sock, &data);

        // Receive length of data as u64;
        let mut rcv_len_u8_arr: [u8; 8] = [0; 8];
        sock_read(sock, &mut rcv_len_u8_arr);
        let rcv_len = u64_of_array(&mut rcv_len_u8_arr);

        // TODO: assert(rcv_len == OUTPUT_SIZE);
        if rcv_len != OUTPUT_SIZE as u64 {
            pr_warn!("rcv_len({}) != OUTPUT_SIZE({})\n", rcv_len, OUTPUT_SIZE as u64);
        }

        // Receive return data as array of u8s.
        let mut rcv_vec_u8: [u8; OUTPUT_SIZE] = [0; OUTPUT_SIZE];
        sock_read(sock, &mut rcv_vec_u8);

        rcv_vec_u8
    }
}

// Needs to be wrapped in a struct to allow for unsafe Send implementation.
#[derive(Clone, Copy)]
struct Filebox(*mut bindings::file);
unsafe impl Send for Filebox {}

struct SharedStateInner {
    // See N_ADDRS for how to index into this
    write_input: Option<[u64; N_ADDRS*2]>,
    filp: Option<Filebox>,
    socket: Socket,
}

struct SharedState {
    inner: Mutex<SharedStateInner>,
}

impl SharedState {
    fn try_new() -> Result<Ref<Self>> {

        let state = Pin::from(UniqueRef::try_new(Self {
            inner: Mutex::new(SharedStateInner {
                write_input: Some([0; N_ADDRS*2]),
                filp: None,
                socket: Socket::new() }),
        })?);

        Ok(state.into())
    }
}

struct Token;
#[vtable]
impl file::Operations for Token {
    type Data = Ref<SharedState>;
    type OpenData = Ref<SharedState>;

    fn open(shared: &Ref<SharedState>, _file: &File) -> Result<Self::Data> {
        Ok(shared.clone())
    }

    // One read call results in dqbuf+qbuf & communicating w/ server for one frame.
    fn read(
        shared: RefBorrow<'_, SharedState>,
        _: &File,
        data: &mut impl IoBufferWriter,
        _offset: u64,
    ) -> Result<usize> {

        let inner = shared.inner.lock();
        let filp = inner.filp.unwrap().0;

        let vidioc_dqbuf: u32 = ioctl_num(IORW, 17, core::mem::size_of::<v4l2_buffer>() as u32);
        let vidioc_qbuf: u32 = ioctl_num(IORW, 15, core::mem::size_of::<v4l2_buffer>() as u32);

        // dqbuf
        xioctl(filp, vidioc_dqbuf, inner.write_input.unwrap()[0]); // buf1 vaddr
        // debug: could print buf.bytesused.
        let index = unsafe {*(inner.write_input.unwrap()[1] as *const v4l2_buffer)}.index;
        let paddr = inner.write_input.unwrap()[4+(index as usize*2)+1];

        // socket write
        let mut image_data = alloc::vec::Vec::try_with_capacity(BUF_SIZE).unwrap();
        for i in 0..BUF_SIZE {
            let b = unsafe{*((paddr + i as u64) as *const u8)};
            image_data.try_push(b).unwrap();
        }
        let mut s = Socket{};
        let out_data = s.analyze(image_data.as_mut_slice());
        data.write_slice(&out_data)?;

        // qbuf
        xioctl(filp, vidioc_qbuf, inner.write_input.unwrap()[0]);

        Ok(OUTPUT_SIZE)

        /*
        // Old version- uses baked in gray image
        let mut gray_data = alloc::vec::Vec::try_with_capacity(BUF_SIZE).unwrap();
        for _ in 0..BUF_SIZE {
            gray_data.try_push(128).unwrap();
        }
        let mut s = Socket{};
        let out_data = s.analyze(gray_data.as_mut_slice());

        data.write_slice(&out_data)?;
        Ok(OUTPUT_SIZE) */

            /* // Works w/ no networking
        let gray_data = [128; OUTPUT_SIZE];
        data.write_slice(&gray_data)?;

        Ok(OUTPUT_SIZE)
        */
    }

    // Called once. Expects to be passed in 4 relevant vaddr/pfn pairs (see N_ADDRS declaration).
    // Opens video file in userspace.
    fn write(
        shared: RefBorrow<'_, SharedState>,
        _: &File,
        data: &mut impl IoBufferReader,
        _offset: u64,
    ) -> Result<usize> {

        // Read data from userspace
        let mut inner = shared.inner.lock();
        let to_read = data.len();
        if to_read != INPUT_SIZE {
            pr_info!("camera write len({}) != expected input size({})\n", to_read, INPUT_SIZE);
            return Err(EINVAL);
        }
        let mut inbuf: [u8; INPUT_SIZE] = [0; INPUT_SIZE];
        data.read_slice(&mut inbuf)?;

        // Parse and write to shared state
        let mut addr_array: [u64; N_ADDRS*2] = [0; N_ADDRS*2];
        for i in 0..INPUT_SIZE {
            let n = i / 8;
            let b = i % 8;
            addr_array[n] |= (inbuf[i] as u64) << ((b*8) as u64);
        }
        pr_info!("camera getting buffer info: vaddr-pfn pairs\n");
        for i in 0..N_ADDRS {
            pr_info!("frame {} vaddr {:x} pfn {:x}\n", i, addr_array[i*2], addr_array[i*2+1]);
        }

        // Modify pfn's to paddr's
        for i in 0..N_ADDRS {
            let vaddr = addr_array[i*2];
            let pfn = addr_array[i*2+1];
            let paddr = (pfn * PAGE_SIZE as u64) + (vaddr % PAGE_SIZE as u64) + (unsafe{bindings::page_offset_base as u64});
            addr_array[i*2+1] = paddr;
        }

        inner.write_input = Some(addr_array);

        // open filp
        let s = CString::try_from_fmt(fmt!("{}", "/dev/video0")).unwrap();
        let filp =
            unsafe {
            bindings::filp_open(
                s.as_char_ptr(),
                (bindings::O_RDWR | bindings::O_NONBLOCK) as i32,
                (bindings::S_IRUSR | bindings::S_IWUSR) as u16
                )
            };
        pr_info!("opened filp: {:x}\n", filp as u64);
        inner.filp = Some(Filebox(filp));

        Ok(to_read)

    }

    // Close filp on release.
    fn release(shared: Ref<SharedState>, _file: &File) {
        let inner = shared.inner.lock();
        match &inner.filp {
            Some(f) => {
                pr_info!("closing filp\n");
                unsafe { bindings::filp_close(f.0, core::ptr::null_mut()) };
            }
            None => {}
        }
    }
}

struct RustCamera {
    _dev: Pin<Box<miscdev::Registration<Token>>>,
}

impl kernel::Module for RustCamera {
    fn init(_name: &'static CStr, _module: &'static ThisModule) -> Result<Self> {
        pr_info!("Starting camera memory module.\n");

        pr_info!("page_offset_base: {:x}\n", bindings::page_offset_base);
        // pr_info!("max_pfn: {}\n", bindings::max_pfn);
        pr_info!("vmalloc_base: {:x}\n", bindings::vmalloc_base);
        pr_info!("vmemmap_base: {:x}\n", bindings::vmemmap_base);

        let state = SharedState::try_new()?;

        Ok(RustCamera {
            _dev: miscdev::Registration::new_pinned(fmt!("kerncamera"), state)?,
        })
    }
}

impl Drop for RustCamera {
    fn drop(&mut self) {
        pr_info!("Ending rust camera module.\n");
    }
}
