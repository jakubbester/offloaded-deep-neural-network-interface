use opencv::core::{CV_8UC3, Mat, /*Size, Mat_AUTO_STEP,*/ VecN};
use opencv::highgui::*;

use nix::libc::{c_int};
// use std::env;
use std::time::Instant;

use nix::fcntl::{open, OFlag};
use nix::sys::stat::Mode;
use nix::unistd::{close, write, read};
use nix::errno::Errno;
use nix::sys::uio::pread;

mod utils;
use utils::*;

mod v4l2;
use v4l2::{VideoHandler, V4L2_MEMORY_MMAP, v4l2_buffer};

// const RCV_VIDEO: bool = false;
const W: usize = 400;
const H: usize = 712;
const OUTPUT_SIZE: usize = 17*4*3;
const V4L2_BUF_TYPE_VIDEO_CAPTURE: usize = 1;
const PAGE_SIZE: usize = 4096;

// Deserialize u64 array to bytes.
fn u8_vec_of_u64_arr(a: &[u64]) -> Vec<u8> {
    let mut ans = vec![];

    for x in a {
        for i in 0..8 {
            ans.push(((x >> (i*8)) & 0xff) as u8);
        }
    }

    ans
}

// Expects length is divisible by 4, may panic otherwise.
fn f32_array_of_u8_array(arr: &[u8]) -> Vec<f32> {
    if arr.len() % 4 != 0 {
        println!("length not divisible by 4: {}", arr.len());
        return vec![];
    }

    let final_len = arr.len() / 4;
    let mut ans = vec![];

    for i in 0..final_len {
        let f = f32::from_le_bytes(<&[u8] as TryInto<[u8; 4]>>::try_into(&arr[i*4..(i*4)+4]).expect("slice conversion failed"));
        ans.push(f);

    }

    ans
}

fn u64_of_u8_arr(a: &[u8]) -> u64 {
    let mut ans: u64 = 0;
    for i in 0..8 {
        ans += (a[i] as u64) << (8*(i as u64));
    }
    ans
}

// https://stackoverflow.com/questions/5748492/is-there-any-api-for-determining-the-physical-address-from-virtual-address-in-li/45128487#45128487
pub fn read_pfn(fd: c_int, vaddr: u64) -> Result<u64, Errno> {
    let mut nread = 0;
    let mut data: [u8; 8] = [0; 8];

    let vpn = vaddr / (PAGE_SIZE as u64);

    while nread < 8 {
        let ret = pread(fd, &mut data[nread..], (vpn*8 + (nread as u64)) as i64)?;
        nread += ret;
        if ret <= 0 {
            println!("pread error in read_pfn: returned {}", ret);
            return Ok(0)
        }
    }

    let entry = u64_of_u8_arr(&data);

    Ok(entry & ((1u64 << 55) - 1))
}


fn main() {
    // IP address is baked into the kernel module now.
    // let addr = env::args().nth(1).expect("Usage: ./cmd <address>");

    let video_handler = VideoHandler::new().unwrap();

    // Acquire address & pfn pairs to pass to kernel.
    // TODO: buf2 is redundant. Make buf1 more stable, ie static or Pinned.
    let fd = open("/proc/self/pagemap", OFlag::O_RDONLY, Mode::S_IRUSR.union(Mode::S_IWUSR)).unwrap();
    let mut buf1 = v4l2_buffer {
           type_: V4L2_BUF_TYPE_VIDEO_CAPTURE as u32,
           memory: V4L2_MEMORY_MMAP as u32,
           ..Default::default()
    };
    let mut buf2 = v4l2_buffer {
           type_: V4L2_BUF_TYPE_VIDEO_CAPTURE as u32,
           memory: V4L2_MEMORY_MMAP as u32,
           ..Default::default()
    };

    let buf1_vaddr = (&buf1 as *const v4l2_buffer) as u64;
    let buf1_pfn = read_pfn(fd, buf1_vaddr).unwrap();
    let buf2_vaddr = (&buf2 as *const v4l2_buffer) as u64;
    let buf2_pfn = read_pfn(fd, buf2_vaddr).unwrap();
    let mmap1_vaddr = video_handler.buffers[0].start as u64;
    let mmap1_pfn = read_pfn(fd, mmap1_vaddr).unwrap();
    let mmap2_vaddr = video_handler.buffers[1].start as u64;
    let mmap2_pfn = read_pfn(fd, mmap2_vaddr).unwrap();
    close(fd).unwrap();

    // Send addresses to kernel via write()
    let fd2 = open("/dev/kerncamera", OFlag::O_RDWR, Mode::S_IRUSR.union(Mode::S_IWUSR)).unwrap();
    let arr = [buf1_vaddr, buf1_pfn, buf2_vaddr, buf2_pfn, mmap1_vaddr, mmap1_pfn, mmap2_vaddr, mmap2_pfn];
    let v = u8_vec_of_u64_arr(&arr);
    let _nbytes = write(fd2, v.as_slice()).unwrap();

    // One iteration per frame, sequentially.
    loop {
        let now = Instant::now();

        // Obtain a frame via read()
        let mut buf: [u8; OUTPUT_SIZE] = [0; OUTPUT_SIZE];
        read(fd2, &mut buf).unwrap();
        let out_points = f32_array_of_u8_array(buf.as_slice());
        let after_interpreter = now.elapsed().as_secs_f64();
        println!("points: {:?}", out_points);

        let output_data = out_points.as_slice();
        let mut mat_video =
            Mat::new_rows_cols_with_default(
                    H as i32,
                    W as i32,
                    CV_8UC3,
                    VecN::new(1.0, 1.0, 1.0, 1.0)
                    ).unwrap();

        // Draw & present annotated frame.
        draw_keypoints(&mut mat_video, output_data, 0.25);
        imshow("MoveNet", &mat_video).expect("imshow [ERROR]");
        let total = now.elapsed().as_secs_f64();
        let after_present = total - after_interpreter;

        // Print benchmarking code.
        let interp_frac = after_interpreter / total * 100.0;
        let present_frac = after_present / total * 100.0;
        println!("total {:.5} | interp {:.5} {:.3}% | present {:.5} {:.3}%",
                 total, after_interpreter, interp_frac, after_present, present_frac);

        // Exit if key pressed.
        let key = wait_key(1).unwrap();
        if key > 0 && key != 255 {
            break;
        }
    }
    close(fd2).unwrap();
}
