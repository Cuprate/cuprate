//---------------------------------------------------------------------------------------------------- Use
use serde::{Serialize,Deserialize};
use disk::Empty;
use std::path::{Path,PathBuf};
use const_format::formatcp;
use once_cell::sync::OnceCell;
use crate::constants::CUPRATE_PROJECT_DIR;

//---------------------------------------------------------------------------------------------------- Constants
// Compile-time zipped bytes of Cuprate documentation.
//
// Should exist in the repo somewhere.
const DOCS_ZIP: &[u8] = todo!(); // include_bytes!("relative repo path to `docs.zip`");

//---------------------------------------------------------------------------------------------------- Docs
// FIXME: disk needs #[allow(non_camel_case_types)]
//
// An empty marker struct representing the documentation on disk.
//
// This struct has associated PATH metadata and
// method helpers for creating/opening documentation.
disk::empty!(Docs, disk::Dir::Data, CUPRATE_PROJECT_DIR, "docs", "__docs");
#[derive(Debug,PartialEq,Eq,PartialOrd,Ord,Serialize,Deserialize)]
pub struct Docs;

impl Docs {
	pub fn create() -> Result<PathBuf, anyhow::Error> {
		let mut path = Self::base_path()?;
		let _ = std::fs::remove_dir_all(&path);
		Self::mkdir()?;

		let mut zip = zip::ZipArchive::new(std::io::Cursor::new(DOCS_ZIP))?;

		// The `ZIP` contains `/docs`, so pop it out.
		path.pop();
		zip.extract(&path)?;
		path.push("docs");

		Ok(path)
	}

	pub fn create_open() -> Result<(), anyhow::Error> {
		match crate::docs::Docs::create() {
			Ok(mut path) => {
				path.push("index.html");
				Ok(open::that_detached(path)?)
			},
			Err(e) => Err(e),
		}
	}
}

//---------------------------------------------------------------------------------------------------- TESTS
//#[cfg(test)]
//mod tests {
//	#[test]
//		fn __TEST__() {
//	}
//}
