use nix::fcntl::{open, OFlag};
use nix::{ioctl_readwrite, ioctl_write_ptr};
use nix::errno::Errno;
use nix::sys::stat::Mode;
use nix::sys::mman::{mmap, munmap, ProtFlags, MapFlags};
use nix::unistd::{close, /*write*/};
use nix::libc::size_t;
use std::ffi::{c_int, c_ulong, c_void};
// use std::ptr::copy_nonoverlapping;
// use std::path::Path;
// use std::fs::File;
// use std::io;
// use std::io::BufRead;

const N_BUFFERS: usize = 2;
const W: usize = 400;
const H: usize = 712;

// fn video_name() -> String {
//     "/dev/video0".to_string()
// }

// fn u8_vec_of_u64_arr(a: &[u64]) -> Vec<u8> {
//     let mut ans = vec![];
// 
//     for x in a {
//         for i in 0..8 {
//             ans.push(((x >> (i*8)) & 0xff) as u8);
//         }
//     }
// 
//     ans
// }

// fn parse_hex_str(a: &str) -> u64 {
//     let mut ans: u64 = 0;
//     for c in a.chars() {
//         ans = ans << 4;
//         ans = ans + (c.to_digit(16).unwrap() as u64);
//     }
//     ans
// }

// Turns out parsing /proc/self/pagemap is unnecessary.
// // Using /proc/self/maps and /proc/self/pagemap to find pfns of virtual addresses.
// // return_value[i*2] is vaddr of frame i
// // return_value[i*2+1] is pfn of frame i
// fn get_virt_addrs() -> Result<[u64; N_BUFFERS*2], Errno> {
//     // parse /proc/self/maps
//     let fname = video_name();
//     let path = Path::new("/proc/self/maps");
//     let file = File::open(&path).unwrap();
//     let lines = io::BufReader::new(file).lines();
// 
//     let mut vaddrs = vec![];
// 
//     for line in lines {
//         let line_str = line.unwrap();
//         let a: Vec<String> = line_str
//             .split_whitespace()
//             .map(|x| x.to_string())
//             .collect();
//         let addr_str = &a[0];
//         let file_str = &a[a.len()-1];
//         let b: Vec<String> = addr_str
//             .split('-')
//             .map(|x| x.to_string())
//             .collect();
//         let start_addr_str = &b[0];
//         let end_addr_str = &b[1];
// 
//         if !fname.eq(file_str) {
//             continue;
//         }
// 
//         let start_addr = parse_hex_str(start_addr_str.as_str());
//         let end_addr = parse_hex_str(end_addr_str.as_str());
// 
//         vaddrs.push((start_addr, end_addr));
// 
//         println!("str : {} {}", start_addr_str, end_addr_str);
//         println!("hex : {:x} {:x}", start_addr, end_addr);
//     }
// 
//     let fd = open("/proc/self/pagemap", OFlag::O_RDONLY, Mode::S_IRUSR.union(Mode::S_IWUSR))?;
// 
//     if vaddrs.len() != N_BUFFERS {
//         println!("WARNING: vaddrs.len({}) != N_BUFFERS({})", vaddrs.len(), N_BUFFERS);
//     }
// 
//     let mut pfns = vec![];
// 
//     let mut prev = 0;
//     for x in vaddrs.iter() {
//         let start = x.0;
// 
//         let result = read_pfn(fd, start).unwrap();
//         println!("vaddr: {:x}", start);
//         println!("pfn  : {:x}", result);
// 
//         pfns.push(result);
// 
//         // It seems individual buffers are continuously mapped
//         /*
//         // TODO: assert % page_size is 0
//         let start_vpn = start / PAGE_SIZE as u64;
//         let end_vpn = end / PAGE_SIZE as u64;
//         println!("s {:x} e {:x}", start_vpn, end_vpn);
// 
//         for vpn in start_vpn..(end_vpn) {
//             let vaddr = vpn * PAGE_SIZE as u64;
//             let result = read_pfn(fd, start).unwrap();
// 
//             if result != prev {
//                 println!("vaddr: {:x}", vaddr);
//                 println!("pfn  : {:x}", result);
//                 prev = result;
//             }
//         } */
//     }
// 
//     let mut ans = [0; N_BUFFERS*2];
//     for i in 0..N_BUFFERS {
//         ans[i*2] = vaddrs[i].0;
//         ans[i*2+1] = pfns[i];
//     }
// 
//     // TODO: doesn't properly handle close in all cases
//     close(fd)?;
// 
//     println!("finished");
// 
//     Ok(ans)
// }

// FFI bindings to v4l2 api

/*
#[repr(C)]
pub struct v4l2_capability {
    pub driver: [u8; 16],
    pub card: [u8; 32],
    pub bus_info: [u8; 32],
    pub version: u32,
    pub capabilities: u32,
    pub device_caps: u32,
    pub reserved: [u32; 3],
}

ioctl_read!(vidioc_querycap, b'V', 0, v4l2_capability);
*/

pub const FORMAT_PADDING: usize = 208 - 10*4;
#[repr(C)]
pub struct v4l2_format {
    pub type_: u32,
    pub space: u32,
    pub width: u32,
    pub height: u32,
    pub pixelformat: u32,
    pub field: u32,
    pub bytesperline: u32,
    pub sizeimage: u32,
    pub colorspace: u32,
    pub priv_: u32,
    pub others: [u8; FORMAT_PADDING]
}

#[repr(C)]
#[derive(Default)]
pub struct v4l2_requestbuffers {
    pub count: u32,
    pub type_: u32,
    pub memory: u32,
    pub capabilities: u32,
    pub flags: u8,
    pub reserved: [u8; 3]
}

#[repr(C)]
pub union anon_plane_m {
    mem_offset: u32,
    userptr: c_ulong,
    fd: i32
}

#[repr(C)]
pub struct v4l2_plane {
    pub bytesused: u32,
    pub length: u32,
    pub m: anon_plane_m,
    pub data_offset: u32,
    pub reserved: [u32; 11]
}

#[repr(C)]
pub union anon_buffer_m {
    pub offset: u32,
    pub userptr: c_ulong,
    pub planes: *mut v4l2_plane,
    pub fd: i32
}

#[repr(C)]
#[derive(Default)]
pub struct v4l2_timecode {
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
pub struct timeval {
    tv_sec: i64,
    tv_usec: i64
}

#[repr(C)]
pub struct v4l2_buffer {
    pub index: u32,
    pub type_: u32,
    pub bytesused: u32,
    pub flags: u32,
    pub field: u32,
    pub timestamp: timeval,
    pub timecode: v4l2_timecode,
    pub sequence: u32,

    // memory location
    pub memory: u32,
    pub m: anon_buffer_m,
    pub length: u32,
    pub reserved2: u32,
    pub anon_union: u32 // technically u32/s32
}

impl Default for v4l2_buffer {
    fn default() -> Self {
        v4l2_buffer {
            index: 0,
            type_: 0,
            bytesused: 0,
            flags: 0,
            field: 0,
            timestamp: timeval{tv_sec: 0, tv_usec: 0},
            timecode: Default::default(),
            sequence: 0,
            memory: 0,
            m: anon_buffer_m { offset: 0 },
            length: 0,
            reserved2: 0,
            anon_union: 0
        }
    }
}

const V4L2_BUF_TYPE_VIDEO_CAPTURE: usize = 1;
pub const V4L2_MEMORY_MMAP: usize = 1;
// const V4L2_PIX_FMT_RGB24: usize = 0x52474233;
// const V4L2_PIX_FMT_YUV420: usize = 0x32315559;
// const V4L2_PIX_FMT_VYUY: usize = 0x56595559;
// const V4L2_FIELD_INTERLACED: usize = 4;
// const V4L2_FIELD_ANY: usize = 4;

ioctl_readwrite!(vidioc_reqbufs, b'V', 8, v4l2_requestbuffers);
ioctl_readwrite!(vidioc_querybuf, b'V', 9, v4l2_buffer);
ioctl_readwrite!(vidioc_qbuf, b'V', 15, v4l2_buffer);
// ioctl_write_int!(vidioc_streamon, b'V', 18);
// ioctl_write_int!(vidioc_streamoff, b'V', 19);
ioctl_readwrite!(vidioc_dqbuf, b'V', 17, v4l2_buffer);
ioctl_readwrite!(vidioc_g_fmt, b'V', 4, v4l2_format);
ioctl_readwrite!(vidioc_s_fmt, b'V', 5, v4l2_format);
ioctl_write_ptr!(vidioc_streamon, b'V', 18, c_int);
ioctl_write_ptr!(vidioc_streamoff, b'V', 19, c_int);

unsafe fn xioctl<T>(myfn: unsafe fn(c_int, *mut T) -> Result<i32, Errno>, fd: c_int,
                    arg: *mut T) -> Result<c_int, Errno> {
    loop {
        let result = myfn(fd, arg);
        match result {
            Ok(retval) => {return Ok(retval);}
            Err(Errno::EINTR) => {}
            Err(Errno::EAGAIN) => {}
            Err(other) => { return Err(other) }
        }
    }
}

unsafe fn xioctl_const<T>(myfn: unsafe fn(c_int, *const T) -> Result<i32, Errno>, fd: c_int,
                    arg: *const T) -> Result<c_int, Errno> {
    loop {
        let result = myfn(fd, arg);
        match result {
            Ok(retval) => {return Ok(retval);}
            Err(Errno::EINTR) => {}
            Err(Errno::EAGAIN) => {}
            Err(other) => { return Err(other) }
        }
    }
}

// Definition of handler struct

#[derive(Copy, Clone)]
pub struct FrameBuffer {
    pub start: *mut c_void,
    length: size_t
}

pub struct VideoHandler {
    fd: c_int,
    pub buffers: [FrameBuffer; N_BUFFERS]
}

impl VideoHandler {
    pub fn new() -> Result<VideoHandler, Errno> {
        let fd = open("/dev/video0", OFlag::O_NONBLOCK.union(OFlag::O_RDWR), Mode::S_IRUSR.union(Mode::S_IWUSR))?;

        let mut gfmt = v4l2_format {
            type_: V4L2_BUF_TYPE_VIDEO_CAPTURE as u32,
            space: 0,
            width: W as u32,
            height: H as u32,
            pixelformat: 0,
            field: 0,
            bytesperline: 0,
            sizeimage: 0,
            colorspace: 0,
            priv_: 0,
            others: [0; FORMAT_PADDING]
        };
        // unsafe { vidioc_g_fmt(fd, &mut gfmt as *mut v4l2_format).unwrap(); }
        unsafe { xioctl(vidioc_g_fmt, fd, &mut gfmt as *mut v4l2_format).unwrap(); }
        println!("gfmt: type {} space {} width {} height {} pixfmt 0x{:x} field {}
            bytesperline {} sizeimage {} colorspace {} priv {}",
                 gfmt.type_,
                 gfmt.space,
                 gfmt.width,
                 gfmt.height,
                 gfmt.pixelformat,
                 gfmt.field,
                 gfmt.bytesperline,
                 gfmt.sizeimage,
                 gfmt.colorspace,
                 gfmt.priv_
        );

    /*
        let mut fmt = v4l2_format {
            type_: V4L2_BUF_TYPE_VIDEO_CAPTURE as u32,
            space: 0,
            width: W as u32,
            height: H as u32,
            pixelformat: V4L2_PIX_FMT_YUV420 as u32,
            field: V4L2_FIELD_ANY as u32,
            bytesperline: 0,
            sizeimage: 0,
            colorspace: 0,
            priv_: 0,
            others: [0; FORMAT_PADDING]
        };
        unsafe { xioctl(vidioc_s_fmt, fd, &mut fmt as *mut v4l2_format).unwrap(); }
        if fmt.pixelformat != V4L2_PIX_FMT_YUV420 as u32 {
            println!("warning: format not vyuy: {}", fmt.pixelformat);
            println!("vyuy: {}", V4L2_PIX_FMT_YUV420);
        } */

        let mut req: v4l2_requestbuffers = Default::default();
        req.count = 2;
        req.type_ = V4L2_BUF_TYPE_VIDEO_CAPTURE as u32;
        req.memory = V4L2_MEMORY_MMAP as u32;
        unsafe {
            xioctl(vidioc_reqbufs, fd, &mut req as *mut v4l2_requestbuffers).unwrap();
        }

        let mut buffers: [FrameBuffer; N_BUFFERS] = [
            FrameBuffer{start: 0 as *mut c_void, length: 0}; N_BUFFERS
        ];

        for i in 0..N_BUFFERS {
            let mut buf = v4l2_buffer {
                index: i as u32,
                type_: V4L2_BUF_TYPE_VIDEO_CAPTURE as u32,
                memory: V4L2_MEMORY_MMAP as u32,
                ..Default::default()
            };

            unsafe { xioctl(vidioc_querybuf, fd, &mut buf as *mut v4l2_buffer).unwrap(); }

            buffers[i].length = buf.length as usize;

            buffers[i].start = unsafe {
                mmap(0 as *mut c_void, buf.length as usize,
                     ProtFlags::PROT_READ.union(ProtFlags::PROT_WRITE),
                     MapFlags::MAP_SHARED,
                     fd, buf.m.offset as i64).unwrap()
            };
        }

        for i in 0..N_BUFFERS {
            let mut buf = v4l2_buffer {
                type_: V4L2_BUF_TYPE_VIDEO_CAPTURE as u32,
                memory: V4L2_MEMORY_MMAP as u32,
                index: i as u32,
                ..Default::default()
            };

            unsafe { xioctl(vidioc_qbuf, fd, &mut buf as *mut v4l2_buffer).unwrap(); }
        }

        let mut type_ = V4L2_BUF_TYPE_VIDEO_CAPTURE as i32;
        unsafe { xioctl_const(vidioc_streamon, fd, &mut type_ as *mut i32).unwrap(); }

        /*
        // Figure out info about pfn's & write to kernel
        let addr_array = get_virt_addrs()?;
        let addr_buf = u8_vec_of_u64_arr(&addr_array);
        assert!(addr_buf.len() == N_BUFFERS*2*8);
        let nbytes = write(kern_fd, addr_buf.as_slice())?;
        assert!(nbytes == addr_buf.len());
        */

        Ok(VideoHandler{fd, buffers})
    }

//     pub fn frame(&self) -> Result<Vec<u8>, Errno> {
//         let mut buf = v4l2_buffer {
//             type_: V4L2_BUF_TYPE_VIDEO_CAPTURE as u32,
//             memory: V4L2_MEMORY_MMAP as u32,
//             ..Default::default()
//         };
// 
//         unsafe { xioctl(vidioc_dqbuf, self.fd, &mut buf as *mut v4l2_buffer)?; }
// 
//         let mut ans_vec = vec![0; buf.bytesused as usize];
// 
//         unsafe {copy_nonoverlapping::<u8>(
//             self.buffers[buf.index as usize].start as *const u8,
//             ans_vec.as_mut_ptr(),
//             buf.bytesused as usize
//         ); }
// 
//         unsafe { xioctl(vidioc_qbuf, self.fd, &mut buf as *mut v4l2_buffer)?; }
// 
//         Ok(ans_vec)
//     }
}

impl Drop for VideoHandler {
    fn drop(&mut self) {
        let mut type_ = V4L2_BUF_TYPE_VIDEO_CAPTURE as i32;
        unsafe { vidioc_streamoff(self.fd, &mut type_ as *mut i32).unwrap(); }
        for i in 0..N_BUFFERS {
            unsafe { munmap(self.buffers[i].start, self.buffers[i].length).unwrap(); }
        }
        close(self.fd).unwrap();
    }
}
