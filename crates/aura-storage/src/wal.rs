use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use crc32fast::Hasher as Crc32;
use parking_lot::Mutex;
use thiserror::Error;

const WAL_MAGIC: &[u8; 4] = b"AWAL";

#[derive(Error, Debug)]
pub enum WalError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid WAL magic header")]
    InvalidMagic,
    #[error("corrupted record")]
    CorruptedRecord,
}

pub struct WriteAheadLog {
    file: Mutex<File>,
    path: PathBuf,
}

impl WriteAheadLog {
    /// Open existing WAL or create a new one.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, WalError> {
        let path = path.as_ref().to_path_buf();
        let exists = path.exists();

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        if !exists || file.metadata()?.len() == 0 {
            file.write_all(WAL_MAGIC)?;
            file.sync_all()?;
        }

        Ok(Self {
            file: Mutex::new(file),
            path,
        })
    }

    /// Append a record to the WAL with CRC32 integrity checksum.
    pub fn append(&self, record: &[u8]) -> Result<(), WalError> {
        let mut crc = Crc32::new();
        crc.update(record);
        let checksum = crc.finalize();

        let len = record.len() as u32;

        let mut file = self.file.lock();
        file.seek(SeekFrom::End(0))?;
        file.write_all(&len.to_be_bytes())?;
        file.write_all(&checksum.to_be_bytes())?;
        file.write_all(record)?;
        file.sync_all()?;
        Ok(())
    }

    /// Recover all valid entries and truncate any corrupt partial trailing entry.
    pub fn recover_and_truncate(&self) -> Result<Vec<Vec<u8>>, WalError> {
        let mut file = self.file.lock();
        file.seek(SeekFrom::Start(0))?;

        let mut magic = [0u8; 4];
        if file.read_exact(&mut magic).is_err() || &magic != WAL_MAGIC {
            return Err(WalError::InvalidMagic);
        }

        let mut records = Vec::new();
        let mut last_valid_offset = 4u64;

        loop {
            let mut len_buf = [0u8; 4];
            if file.read_exact(&mut len_buf).is_err() {
                break;
            }
            let len = u32::from_be_bytes(len_buf) as usize;

            let mut crc_buf = [0u8; 4];
            if file.read_exact(&mut crc_buf).is_err() {
                break;
            }
            let expected_crc = u32::from_be_bytes(crc_buf);

            let mut payload = vec![0u8; len];
            if file.read_exact(&mut payload).is_err() {
                break;
            }

            let mut crc = Crc32::new();
            crc.update(&payload);
            if crc.finalize() != expected_crc {
                break;
            }

            last_valid_offset += 8 + len as u64;
            records.push(payload);
        }

        // Truncate cleanly at last valid offset
        file.set_len(last_valid_offset)?;
        file.seek(SeekFrom::End(0))?;

        Ok(records)
    }
}
