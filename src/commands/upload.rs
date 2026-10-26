//! Uploading a folder for a container order, like the console: pack it as a tar (folders and
//! regular files only, uid/gid 1000, modes 755 / 644, executables keep 755), then
//! POST /api/v1/market/containers/uploads {size_bytes} -> PUT .../{id}?offset=N (chunks the API
//! sizes, resumed from `received_bytes` after an error) -> POST .../{id}/complete.
//! Symlinks and special files are skipped (the API refuses them); the order then names
//! `{upload_id, mount_path}` in `files`.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde_json::{json, Value};

use crate::api::Client;
use crate::error::{CliError, ErrorKind};

/// The API's limit for one folder (packed).
const MAX_BYTES: u64 = 1 << 30;
const RETRIES: u32 = 3;

pub struct Uploaded {
    pub upload_id: String,
    pub files: u64,
    pub bytes: u64,
    pub skipped: Vec<String>,
}

/// Packs `dir` and uploads it; returns the upload's id.
pub fn upload_folder(client: &Client, dir: &Path) -> Result<Uploaded, CliError> {
    if !dir.is_dir() {
        return Err(CliError::usage(format!("{} isn't a folder", dir.display())));
    }
    let mut tar = tempfile::tempfile().map_err(io("make a temporary file"))?;
    let (files, skipped) = pack(dir, &mut tar)?;
    let size = tar.seek(SeekFrom::End(0)).map_err(io("measure the archive"))?;
    if size > MAX_BYTES {
        return Err(CliError::usage(format!(
            "{} is {} MB packed; a folder can be at most 1 GB",
            dir.display(),
            size >> 20
        )));
    }

    let start = client.post("/api/v1/market/containers/uploads", Some(json!({ "size_bytes": size })))?;
    let id = start["id"].as_str().ok_or_else(|| internal("the API didn't give the upload an id"))?.to_string();
    let chunk = start["chunk_bytes"].as_u64().filter(|c| *c > 0).unwrap_or(960 << 10);

    let mut offset = 0u64;
    let mut failures = 0u32;
    let mut buf = vec![0u8; chunk as usize];
    while offset < size {
        let len = chunk.min(size - offset) as usize;
        tar.seek(SeekFrom::Start(offset)).map_err(io("read the archive"))?;
        tar.read_exact(&mut buf[..len]).map_err(io("read the archive"))?;
        let sent = client.put_bytes(
            &format!("/api/v1/market/containers/uploads/{id}"),
            &[("offset", &offset.to_string())],
            &buf[..len],
        );
        match sent {
            Ok(answer) => {
                offset = answer["received_bytes"].as_u64().unwrap_or(offset + len as u64);
                failures = 0;
            }
            // A dropped connection: ask nothing, resend from where the API says it got to.
            Err(e) if e.kind == ErrorKind::Network && failures < RETRIES => failures += 1,
            Err(e) => return Err(e),
        }
    }
    client.post(&format!("/api/v1/market/containers/uploads/{id}/complete"), None)?;
    Ok(Uploaded { upload_id: id, files, bytes: size, skipped })
}

/// Writes `dir`'s contents (not the folder itself) into `out` as a tar. Returns the number of
/// files and the paths skipped.
fn pack(dir: &Path, out: &mut File) -> Result<(u64, Vec<String>), CliError> {
    let mut builder = tar::Builder::new(out);
    let mut files = 0u64;
    let mut skipped = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), String::new())];
    while let Some((path, prefix)) = stack.pop() {
        let mut entries: Vec<_> = fs::read_dir(&path)
            .map_err(io("read the folder"))?
            .collect::<Result<_, _>>()
            .map_err(io("read the folder"))?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
            // symlink_metadata: a symlink is skipped, not followed out of the folder.
            let meta = fs::symlink_metadata(entry.path()).map_err(io("read the folder"))?;
            if meta.is_dir() {
                append(&mut builder, &rel, tar::EntryType::Directory, 0o755, &[])?;
                stack.push((entry.path(), rel));
            } else if meta.is_file() {
                let data = fs::read(entry.path()).map_err(io("read a file"))?;
                append(&mut builder, &rel, tar::EntryType::Regular, file_mode(&meta), &data)?;
                files += 1;
            } else {
                skipped.push(rel);
            }
        }
    }
    builder.finish().map_err(io("write the archive"))?;
    Ok((files, skipped))
}

fn append(
    builder: &mut tar::Builder<&mut File>,
    path: &str,
    kind: tar::EntryType,
    mode: u32,
    data: &[u8],
) -> Result<(), CliError> {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(kind);
    header.set_mode(mode);
    header.set_uid(1000);
    header.set_gid(1000);
    header.set_size(data.len() as u64);
    header.set_mtime(0);
    let name = if kind == tar::EntryType::Directory { format!("{path}/") } else { path.to_string() };
    builder.append_data(&mut header, name, data).map_err(io("write the archive"))
}

#[cfg(unix)]
fn file_mode(meta: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    if meta.permissions().mode() & 0o111 != 0 {
        0o755
    } else {
        0o644
    }
}

#[cfg(not(unix))]
fn file_mode(_: &fs::Metadata) -> u32 {
    0o644
}

fn io(what: &'static str) -> impl Fn(std::io::Error) -> CliError {
    move |e| CliError::new(ErrorKind::Internal, format!("can't {what}: {e}"))
}

fn internal(message: &str) -> CliError {
    CliError::new(ErrorKind::Internal, message)
}

/// `./site:/usr/share/nginx/html` -> ("./site", "/usr/share/nginx/html"). Windows paths
/// (C:\site:/data) split at the last `:` before a `/`.
pub fn parse_upload(text: &str) -> Result<(String, String), CliError> {
    let at = text.rfind(":/").ok_or_else(|| {
        CliError::usage(format!(
            "--upload is LOCAL_DIR:/mount/path, e.g. ./site:/usr/share/nginx/html (got \"{text}\")"
        ))
    })?;
    let (dir, mount) = (&text[..at], &text[at + 1..]);
    if dir.is_empty() {
        return Err(CliError::usage("--upload needs a local folder before the ':'"));
    }
    Ok((dir.to_string(), mount.to_string()))
}

/// What a finished upload looks like in JSON output.
pub fn describe(u: &Uploaded, mount: &str) -> Value {
    json!({ "upload_id": u.upload_id, "mount_path": mount, "files": u.files, "bytes": u.bytes, "skipped": u.skipped })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_flags_parse() {
        assert_eq!(parse_upload("./site:/var/www").unwrap(), ("./site".into(), "/var/www".into()));
        assert_eq!(parse_upload(r"C:\site:/data").unwrap(), (r"C:\site".into(), "/data".into()));
        assert!(parse_upload("./site").is_err());
        assert!(parse_upload(":/data").is_err());
    }

    #[test]
    fn packs_files_and_folders_only() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("index.html"), "hi").unwrap();
        fs::write(dir.path().join("sub/a.txt"), "a").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc/passwd", dir.path().join("link")).unwrap();
        let mut out = tempfile::tempfile().unwrap();
        let (files, skipped) = pack(dir.path(), &mut out).unwrap();
        assert_eq!(files, 2);
        #[cfg(unix)]
        assert_eq!(skipped, vec!["link".to_string()]);
        out.seek(SeekFrom::Start(0)).unwrap();
        let mut archive = tar::Archive::new(out);
        let mut names: Vec<String> =
            archive.entries().unwrap().map(|e| e.unwrap().path().unwrap().display().to_string()).collect();
        names.sort();
        assert_eq!(names, vec!["index.html", "sub/", "sub/a.txt"]);
    }
}
