//! Test-only cryptographic source preservation witness.
use sha2::{Digest, Sha256};
use std::io::{self, Read};
use std::path::Path;

const CHUNK_BYTES: usize = 64 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct FileWitness {
    len: u64,
    sha256: [u8; 32],
}

pub(super) fn capture(path: &Path) -> io::Result<FileWitness> {
    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    let witness = from_reader(&mut file)?;
    if witness.len != len || file.metadata()?.len() != len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "source size changed while hashing",
        ));
    }
    Ok(witness)
}

fn from_reader(reader: &mut impl Read) -> io::Result<FileWitness> {
    let mut buffer = [0u8; CHUNK_BYTES];
    let mut len = 0u64;
    let mut digest = Sha256::new();
    loop {
        let count = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        len = len
            .checked_add(count as u64)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "source size overflow"))?;
        digest.update(&buffer[..count]);
    }
    Ok(FileWitness {
        len,
        sha256: digest.finalize().into(),
    })
}

#[test]
fn file_witness_detects_same_size_mutation_and_length_change() {
    use std::io::{Seek, SeekFrom, Write};
    let path = super::directory("file-witness-mutation");
    let source = path.join("source");
    std::fs::write(&source, b"source bytes").unwrap();
    let before = capture(&source).unwrap();
    assert_eq!(capture(&source).unwrap(), before);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(&source)
        .unwrap();
    file.seek(SeekFrom::Start(1)).unwrap();
    file.write_all(b"X").unwrap();
    file.flush().unwrap();
    let changed = capture(&source).unwrap();
    assert_eq!(changed.len, before.len);
    assert_ne!(changed.sha256, before.sha256);
    file.set_len(before.len + 1).unwrap();
    assert_ne!(capture(&source).unwrap(), changed);
    drop(file);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn file_witness_reads_large_sparse_source_in_bounded_chunks() {
    struct BoundedReader {
        file: std::fs::File,
        reads: usize,
    }
    impl Read for BoundedReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            assert!(
                buffer.len() <= CHUNK_BYTES,
                "source witness read exceeded fixed chunk"
            );
            self.reads += 1;
            self.file.read(buffer)
        }
    }
    let path = super::directory("file-witness-sparse");
    let source = path.join("source");
    let len = 129 * 1024 * 1024 + 17;
    std::fs::File::create(&source)
        .unwrap()
        .set_len(len)
        .unwrap();
    let mut reader = BoundedReader {
        file: std::fs::File::open(&source).unwrap(),
        reads: 0,
    };
    let witness = from_reader(&mut reader).unwrap();
    assert_eq!(witness.len, len);
    assert!(reader.reads >= len as usize / CHUNK_BYTES);
    std::fs::remove_dir_all(path).unwrap();
}
