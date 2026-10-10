use crate::control::{self, Request};
use rom::{Actor, Error, Runtime};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

/// Private Linux fixture mailbox. No network handler exposes these controls.
pub struct Mailbox {
    directory: PathBuf,
    device: u64,
    inode: u64,
    next: u8,
}
impl Mailbox {
    pub fn new(directory: &str) -> rom::Result<Self> {
        let path = Path::new(directory);
        let metadata = fs::symlink_metadata(path).map_err(|_| Error::Denied)?;
        let root = Path::new("/var/tmp/rom-010-authentik-20261007/run/volume");
        if !path.starts_with(root.join("private"))
            || !metadata.is_dir()
            || fs::canonicalize(path).map_err(|_| Error::Denied)? != path
            || metadata.dev() != fs::metadata(root).map_err(|_| Error::Denied)?.dev()
        {
            return Err(Error::Denied);
        }
        Ok(Self {
            directory: path.into(),
            device: metadata.dev(),
            inode: metadata.ino(),
            next: 1,
        })
    }
    pub async fn poll(
        &mut self,
        runtime: &Runtime,
        actor: &Actor,
        subject: &str,
    ) -> rom::Result<bool> {
        let metadata = fs::symlink_metadata(&self.directory).map_err(|_| Error::Denied)?;
        if !metadata.is_dir() || metadata.dev() != self.device || metadata.ino() != self.inode {
            return Err(Error::Denied);
        }
        let mut count = 0;
        for entry in fs::read_dir(&self.directory).map_err(|_| Error::Denied)? {
            count += 1;
            if count > 32 {
                return Err(Error::Denied);
            }
            let entry = entry.map_err(|_| Error::Denied)?;
            if !entry.file_type().map_err(|_| Error::Denied)?.is_file() {
                return Err(Error::Denied);
            }
            let name = entry.file_name().into_string().map_err(|_| Error::Denied)?;
            let (response, index) = if let Some(value) = name.strip_prefix("request-") {
                (false, value)
            } else if let Some(value) = name.strip_prefix("response-") {
                (true, value)
            } else {
                return Err(Error::Denied);
            };
            let index = index.strip_suffix(".json").ok_or(Error::Denied)?;
            let sequence: u8 = index.parse().map_err(|_| Error::Denied)?;
            if !(1..=16).contains(&sequence)
                || index != format!("{sequence:02}")
                || sequence > self.next
                || (response && sequence >= self.next)
            {
                return Err(Error::Denied);
            }
        }
        if self.next > 16 {
            return Ok(false);
        }
        let path = self
            .directory
            .join(format!("request-{:02}.json", self.next));
        // Linux O_NOFOLLOW; the admitted execution profile is Linux/amd64.
        let file = match OpenOptions::new()
            .read(true)
            .custom_flags(0o400000)
            .open(&path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(_) => return Err(Error::Denied),
        };
        let metadata = file.metadata().map_err(|_| Error::Denied)?;
        if !metadata.is_file() || metadata.dev() != self.device {
            return Err(Error::Denied);
        }
        let mut bytes = Vec::new();
        file.take(4097)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Denied)?;
        let request = Request::parse(&bytes)?;
        if request.sequence() != self.next {
            return Err(Error::Denied);
        }
        let revision = control::apply(runtime, actor, subject, &request).await?;
        let response = serde_json::to_vec(
            &serde_json::json!({"sequence":self.next,"revision":revision,"status":"committed"}),
        )
        .map_err(|_| Error::Denied)?;
        let path = self
            .directory
            .join(format!("response-{:02}.json", self.next));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(|_| Error::Denied)?;
        file.write_all(&response).map_err(|_| Error::Denied)?;
        file.sync_all().map_err(|_| Error::Denied)?;
        self.next += 1;
        Ok(true)
    }
}
