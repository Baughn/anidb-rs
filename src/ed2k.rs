pub struct Ed2kHash;
use crypto::digest::Digest;
use errors::{Result};
use md4::Md4;
use std::fs::File;
use std::io::Read;
use std::path::Path;

const BLOCKSIZE: usize = 9500 * 1024;

impl Ed2kHash {
    pub fn hash_bin(filename: &Path) -> Result<[u8; 16]> {
        let mut md4_digest = [0; 16];

        let mut file = File::open(filename)?;
        let file_info = file.metadata()?;
        let file_size = file_info.len() as usize;

        let mut temp_buffer = vec![0; BLOCKSIZE].into_boxed_slice();
        let mut ctx_f = Md4::new();

        let mut blocks = file_size / BLOCKSIZE;
        if file_size % BLOCKSIZE > 0 {
            blocks += 1;
        }

        for _ in 0..blocks {
            let mut ctx_i = Md4::new();

            let read_size = file.read(&mut temp_buffer)?;

            ctx_i.input(&temp_buffer[..read_size]);
            ctx_i.result(&mut md4_digest);

            ctx_f.input(&md4_digest);
        }

        if blocks > 1 {
            ctx_f.result(&mut md4_digest);
        }

        Ok(md4_digest)
    }

    pub fn hash_hex(filename: &Path) -> Result<String> {
        let hash = Ed2kHash::hash_bin(filename)?;
        let mut ret = String::with_capacity(32);
        for hex in hash.iter() {
            ret.push_str(&format!("{:02x}", hex));
        }
        Ok(ret)
    }
}

