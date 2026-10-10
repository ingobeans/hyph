use std::{fs::File, path::Path};

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use walkdir::WalkDir;
use zip::{result::ZipError, write::SimpleFileOptions};

pub const PORT: u16 = 56863;
pub const MAX_NAME_LENGTH: usize = 32;

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq, Eq)]
pub enum ClientMessageType {
    /// Simple ping to verify that server exists and is accessible
    Ping,
    /// Sent to register the client as actively taking the test
    Register,
    /// Transmits large blocks (many MBs) of data
    Transmit,
}

#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq, Eq)]
pub enum ServerMessageType {
    /// Reply to client's ping
    Pong,
    /// Reply to client's register
    RegisterStatus,
    /// Reply to client's transmit
    TransmitSuccess,
}

pub fn parse_message_type<T: FromPrimitive>(value: u8) -> std::io::Result<T> {
    let Some(message_type) = T::from_u8(value) else {
        return std::io::Result::Err(std::io::Error::other(format!(
            "Unknown message type: {value}"
        )));
    };
    Ok(message_type)
}

// joinked directly from the zip crate examples: https://github.com/zip-rs/zip2/blob/master/examples/write_dir.rs#L78
pub fn zip_dir(
    src_dir: &Path,
    dest_file: &Path,
    method: zip::CompressionMethod,
) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(src_dir).is_dir() {
        return Err(ZipError::FileNotFound.into());
    }

    let file = File::create(dest_file)?;

    let walkdir = WalkDir::new(src_dir);

    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(method)
        .unix_permissions(0o755);

    for entry_result in walkdir.into_iter() {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(e) => {
                return Err(format!("Error while traversing directory {src_dir:?}: {e}").into());
            }
        };
        let path = entry.path();
        let path_stripped = path.strip_prefix(src_dir)?;
        let path_as_string = path_stripped
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("{:?} is a Non UTF-8 Path", path_stripped.display()))?;

        if path.is_file() {
            zip.start_file(path_as_string, options)?;
            let mut f = File::open(path)?;

            std::io::copy(&mut f, &mut zip)?;
        } else if !path_stripped.as_os_str().is_empty() {
            zip.add_directory(path_as_string, options)?;
        }
    }
    zip.finish()?;
    Ok(())
}
