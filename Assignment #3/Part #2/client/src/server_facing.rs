use std::io::{Error, ErrorKind};
use std::io::prelude::*;
use std::convert::TryInto;
use std::net::TcpStream;

const RCV_VIDEO: bool = false;

/**
 * Collection of diy serialization/deserialization functions between u64, arrays/vecs of floats to
 * arrays/vecs of u8s for sending over the network. Simpler than learning the serde crate.
 *
 * multi-byte types use little endian byte order.
 */

fn u8_array_of_u64(x: u64) -> Vec<u8> {
    let mut ans = vec![];

    for i in 0..8 {
        ans.push(((x >> (8*i)) % 256) as u8);
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

pub struct Handler {
    stream: TcpStream
}

/**
 * Communication protocol: open connection once at start of the client. For every frame:
 * - client sends length of data as u64
 * - client sends data as u8 stream
 * - server sends length of result data as u64
 * - server sends data as u8 stream
 */
impl Handler {
    // Take address as a constructor argument
    pub fn new(addr: String) -> std::io::Result<Handler> {
        let stream = TcpStream::connect(addr)?;

        Ok(Handler { stream: stream })
    }

    pub fn analyze(&mut self, data:&[u8]) -> std::io::Result<(Vec<u8>, Vec<f32>)> {
        // Send length of data as u64, then send data.
        let mut len_array = u8_array_of_u64(data.len() as u64);
        self.stream.write_all(len_array.as_mut_slice())?;
        self.stream.write_all(data)?;

        // Receive length of return data as u64.
        let mut rcv_len_u8_arr: [u8; 8] = [0; 8];
        self.stream.read_exact(&mut rcv_len_u8_arr)?;
        let rcv_len = u64_of_array(&mut rcv_len_u8_arr);

        // Receive return data as array of u8s.
        let mut rcv_vec_u8: Vec<u8> = vec![0; rcv_len as usize];
        self.stream.read_exact(rcv_vec_u8.as_mut_slice())?;

        if RCV_VIDEO {
            // Split result into video data and point data.
            if data.len() % 2 != 0 {
                return Err(Error::new(ErrorKind::Other, "video data not divisible by 2"));
            }
            // let rcv_video_len = data.len() + (data.len() >> 1);
            let rcv_video_len = data.len() * 2;
            if rcv_vec_u8.len() < rcv_video_len {
                return Err(Error::new(ErrorKind::Other, "rcv data too small"));
            }
            let pt_data = rcv_vec_u8.split_off(rcv_video_len);

            // Convert return data to array of f32s.
            let rcv_vec = f32_array_of_u8_array(pt_data.as_slice());

            Ok((rcv_vec_u8, rcv_vec))
        } else {
            let rcv_vec = f32_array_of_u8_array(rcv_vec_u8.as_slice());

            Ok((vec![], rcv_vec))
        }
    }
}
