//! Downloading Monerod Module
//!
//! This module handles finding the right monerod file to download, downloading it and extracting it.
//!
use std::{
    env::{
        consts::{ARCH, OS},
        current_dir,
    },
    fs::read_dir,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use bytes::Buf;
use reqwest::{get, Error as ReqError};
use tokio::sync::Mutex;

use super::MONEROD_VERSION;

/// A mutex to make sure only one thread at a time downloads monerod.
static DOWNLOAD_MONEROD_MUTEX: Mutex<()> = Mutex::const_new(());

/// Returns the file name to download and the expected extracted folder name.
fn file_name(version: &str) -> (String, String) {
    let download_file = match (OS, ARCH) {
        ("windows", "x64" | "x86_64") => format!("monero-win-x64-{version}.zip"),
        ("windows", "x86") => format!("monero-win-x86-{version}.zip"),
        ("linux", "x64" | "x86_64") => format!("monero-linux-x64-{version}.tar.bz2"),
        ("linux", "x86") => format!("monero-linux-x86-{version}.tar.bz2"),
        ("macos", "x64" | "x86_64") => format!("monero-mac-x64-{version}.tar.bz2"),
        _ => panic!("Can't get monerod for {OS}, {ARCH}."),
    };

    let extracted_dir = match (OS, ARCH) {
        ("windows", "x64" | "x86_64") => {
            format!("monero-x86_64-w64-mingw32-{version}")
        }
        ("windows", "x86") => format!("monero-i686-w64-mingw32-{version}"),
        ("linux", "x64" | "x86_64") => format!("monero-x86_64-linux-gnu-{version}"),
        ("linux", "x86") => format!("monero-i686-linux-gnu-{version}"),
        ("macos", "x64" | "x86_64") => {
            format!("monero-x86_64-apple-darwin11-{version}")
        }
        _ => panic!("Can't get monerod for {OS}, {ARCH}."),
    };

    (download_file, extracted_dir)
}

/// Downloads the monerod file provided, extracts it and puts the extracted folder into `path_to_store`.
async fn download_monerod(file_name: &str, path_to_store: &Path) -> Result<(), ReqError> {
    let res = get(format!("https://downloads.getmonero.org/cli/{file_name}")).await?;
    let monerod_archive = res.bytes().await.unwrap();

    #[cfg(unix)]
    {
        let bzip_decomp = bzip2::read::BzDecoder::new(monerod_archive.reader());
        let mut tar_archive = tar::Archive::new(bzip_decomp);
        tar_archive.unpack(path_to_store).unwrap();
    }
    #[cfg(windows)]
    {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(monerod_archive.as_ref())).unwrap();
        zip.extract(path_to_store).unwrap();
    }

    Ok(())
}

/// Finds the `target` directory, this will work up from the current directory until
/// it finds a `target` directory.
fn find_target() -> PathBuf {
    let mut current_dir = current_dir().unwrap();
    loop {
        let potential_target = current_dir.join("target");
        if read_dir(current_dir.join("target")).is_ok() {
            return potential_target;
        } else if !current_dir.pop() {
            panic!("Could not find ./target");
        }
    }
}

/// Checks if we have monerod or downloads it if we don't and then returns the path to it.
#[allow(clippy::redundant_pub_crate)]
pub(crate) async fn check_download_monerod() -> Result<PathBuf, ReqError> {
    // make sure no other threads are downloading monerod at the same time.
    let _guard = DOWNLOAD_MONEROD_MUTEX.lock().await;

    let path_to_store = find_target();

    let (file_name, dir_name) = file_name(MONEROD_VERSION);

    let path_to_monerod = path_to_store.join(dir_name);

    // Check if we already have monerod
    if read_dir(&path_to_monerod).is_ok() {
        return Ok(path_to_monerod.join("monerod"));
    }

    download_monerod(&file_name, &path_to_store).await?;

    Ok(path_to_monerod.join("monerod"))
}
