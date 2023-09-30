use crc;
use std::io::Read;

const CKSUM: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_CKSUM);

pub(super) fn cksum(file: &mut std::fs::File) -> Option<u32> {
    let mut digest = CKSUM.digest();
    let mut buffer = vec![0; super::BUFFER_SIZE];
    let mut size_total = 0;
    loop {
        let size_chunk = match file.read(&mut buffer) {
            Ok(size) => size,
            Err(e) => {
                eprintln!("Failed to read file: {}", e);
                return None
            },
        };
        if size_chunk == 0 {
            break
        }
        let chunk = &buffer[0..size_chunk];
        digest.update(chunk);
        size_total += size_chunk;
    }
    let mut size_oct = Vec::<u8>::new();
    if size_total > 0 {
        while size_total > 0 {
            size_oct.push((size_total & 0xFF).try_into().unwrap());
            size_total >>= 8;
        }
    } else {
        size_oct.push(0);
    }
    digest.update(&size_oct);
    Some(digest.finalize())
}
