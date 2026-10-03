use rom::{Error, Result};
use serde::{Serialize, de::DeserializeOwned};
use std::io::Write;

pub(crate) fn decode<T: DeserializeOwned>(data: &str) -> Result<T> {
    serde_json::from_str(data).map_err(|_| Error::Storage)
}
pub(crate) fn encode<T: Serialize>(data: &T, limit: usize) -> Result<Vec<u8>> {
    struct Buffer {
        bytes: Vec<u8>,
        limit: usize,
        full: bool,
    }
    impl Write for Buffer {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            if self
                .bytes
                .len()
                .checked_add(data.len())
                .is_none_or(|n| n > self.limit)
            {
                self.full = true;
                return Err(std::io::Error::other("archive limit"));
            }
            self.bytes.extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut out = Buffer {
        bytes: vec![],
        limit,
        full: false,
    };
    if serde_json::to_writer(&mut out, data).is_err() {
        return Err(if out.full {
            Error::TooLarge
        } else {
            Error::Storage
        });
    }
    Ok(out.bytes)
}
