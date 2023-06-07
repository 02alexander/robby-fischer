use std::io::{ErrorKind, Read, Result, Error};
use std::process::{Child, Command, Stdio};

use serde::de::DeserializeOwned;
use serde_json::Deserializer;

pub fn request_streaming<T: DeserializeOwned>(
    url: &str,
) -> Result<impl Iterator<Item = Result<T>>> {
    let process = Command::new("curl")
        .arg("-sN")
        .arg("--")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;

    let process = StoppingProcess { process };

    Ok(Deserializer::from_reader(process)
        .into_iter()
        .map(|r| r.map_err(Into::into)))
}

pub fn request_one<T: DeserializeOwned>(url: &str) -> Result<T> {
    request_streaming(url)?
        .next()
        .ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "expected a single JSON object"))?
}

struct StoppingProcess {
    process: Child,
}

impl Read for StoppingProcess {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.process.stdout.as_mut().unwrap().read(buf)
    }
}

impl Drop for StoppingProcess {
    fn drop(&mut self) {
        _ = self.process.kill();
    }
}
