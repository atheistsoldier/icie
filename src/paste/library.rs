use crate::{
	paste::{
		logic::{Library, Piece}, qpaste_doc_error
	}, util::{fs, path::Path}
};
use evscode::{E, R};
use futures::lock::{Mutex, MutexGuard};
use once_cell::sync::Lazy;
use std::collections::HashMap;

pub static CACHED_LIBRARY: Lazy<LibraryCache> = Lazy::new(LibraryCache::new);

// TODO: Refactor to Option<Path>
/// Path to your competitive programming library for use with the Alt+[ quickpasting feature. Press Alt+[ with this not
/// set to see how to set up this functionality.
#[evscode::config]
static PATH: evscode::Config<Path> = "";

pub struct LibraryCache {
	lock: Mutex<Library>,
}

impl LibraryCache {
	pub fn new() -> LibraryCache {
		LibraryCache {
			lock: Mutex::new(Library { directory: Path::from_native(String::new()), pieces: HashMap::new() }),
		}
	}

	#[allow(clippy::extra_unused_lifetimes)]
	pub async fn update(&'static self) -> R<MutexGuard<'_, Library>> {
		let mut lib = self.lock.lock().await;
		let directory = self.get_directory().await?;
		if directory != lib.directory {
			lib.pieces = HashMap::new();
		}
		let mut new_pieces = HashMap::new();
		for path in fs::read_dir(&directory).await? {
			let id = path.without_extension().fmt_relative(&directory);
			if path.extension() == Some("cpp".to_owned()) {
				let piece = self.maybe_load_piece(path, &id, &mut lib.pieces).await?;
				new_pieces.insert(id, piece);
			}
		}
		lib.directory = directory;
		lib.pieces = new_pieces;
		if lib.pieces.is_empty() {
			return Err(qpaste_doc_error(E::error("library is empty")));
		}
		lib.verify()?;
		Ok(lib)
	}

	async fn maybe_load_piece(&self, path: Path, id: &str, cached_pieces: &mut HashMap<String, Piece>) -> R<Piece> {
		let modified = fs::metadata(&path).await?.modified;
		match cached_pieces.remove(id) {
			Some(cached) if cached.modified == modified => Ok(cached),
			_ => {
				let code = fs::read_to_string(&path).await?;
				let piece = Piece::parse(&code, id.to_owned(), modified).map_err(qpaste_doc_error)?;
				Ok(piece)
			},
		}
	}

	async fn get_directory(&self) -> R<Path> {
		let dir = PATH.get();
		if dir.as_str() == "" {
			return Err(qpaste_doc_error(E::error("library not found")));
		}
		if !fs::exists(&dir).await? {
			return Err(qpaste_doc_error(E::error(format!("library directory {} does not exist", dir))));
		}
		Ok(dir)
	}
}
