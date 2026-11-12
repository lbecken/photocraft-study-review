//! A text document opened from a VFS.

use std::fmt;

use kiln_text::Buffer;

use crate::encoding::{DecodeError, Encoding, decode, encode};
use crate::path::RelPath;
use crate::vfs::{Vfs, VfsError};

/// Why a document could not be opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenError {
    /// The file could not be read.
    Vfs(VfsError),
    /// The file's bytes are not valid in their detected encoding.
    Decode {
        /// The file that failed to decode.
        path: RelPath,
        /// Where decoding failed.
        error: DecodeError,
    },
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::Vfs(e) => write!(f, "cannot open {e}"),
            OpenError::Decode { path, error } => write!(f, "cannot open {path}: {error}"),
        }
    }
}

impl std::error::Error for OpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OpenError::Vfs(e) => Some(e),
            OpenError::Decode { error, .. } => Some(error),
        }
    }
}

/// A file's text in a buffer, plus what is needed to write it back unchanged.
///
/// Line endings are kept verbatim: the buffer holds exactly the file's `\n`, `\r\n` and `\r`.
#[derive(Clone, Debug)]
pub struct Document {
    path: RelPath,
    encoding: Encoding,
    buffer: Buffer,
    /// The text last opened or successfully saved (dirty is judged by content, D-1).
    saved_text: String,
}

impl Document {
    /// Read and decode `path` from `vfs`. Undecodable input is refused, never repaired.
    pub fn open(vfs: &dyn Vfs, path: &RelPath) -> Result<Document, OpenError> {
        let bytes = vfs.read(path).map_err(OpenError::Vfs)?;
        let (text, encoding) = decode(&bytes).map_err(|error| OpenError::Decode {
            path: path.clone(),
            error,
        })?;
        Ok(Document {
            path: path.clone(),
            encoding,
            buffer: Buffer::from_text(&text),
            saved_text: text,
        })
    }

    /// The path the document was opened from and saves to.
    pub fn path(&self) -> &RelPath {
        &self.path
    }

    /// The encoding detected at open, used for save.
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// The document's text.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// The document's text, for editing.
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    /// True iff the buffer text differs from the text last opened or successfully saved.
    pub fn is_dirty(&self) -> bool {
        self.buffer.len_bytes() != self.saved_text.len() || self.buffer.text() != self.saved_text
    }

    /// Write encode(buffer text, encoding()) to path() with write_atomic; returns bytes written.
    /// Writes even when clean. On Err the document is unchanged (still dirty if it was).
    pub fn save(&mut self, vfs: &mut dyn Vfs) -> Result<usize, VfsError> {
        let text = self.buffer.text();
        let bytes = encode(&text, self.encoding);
        vfs.write_atomic(&self.path, &bytes)?;
        self.saved_text = text;
        Ok(bytes.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryFs;

    fn p(s: &str) -> RelPath {
        RelPath::new(s).unwrap()
    }

    #[test]
    fn unedited_round_trip_is_byte_exact_for_each_encoding() {
        let corpus: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"a\r\nb\nc\rd\r\n\r\n\n\r".to_vec(),
            "no final newline 😀 é\0".as_bytes().to_vec(),
            vec![0xEF, 0xBB, 0xBF],
            vec![0xEF, 0xBB, 0xBF, b'x', b'\r', b'\n'],
            encode("a😀\r\n", Encoding::Utf16Le),
            encode("a😀\r\n", Encoding::Utf16Be),
        ];
        for (i, bytes) in corpus.iter().enumerate() {
            let path = p(&format!("f{i}"));
            let mut fs = MemoryFs::new();
            fs.insert(&path, bytes);
            let mut doc = Document::open(&fs, &path).unwrap();
            assert!(!doc.is_dirty());
            assert!(!doc.buffer().text().starts_with('\u{feff}'));
            assert_eq!(doc.save(&mut fs).unwrap(), bytes.len());
            assert_eq!(fs.get(&path).as_ref(), Some(bytes));
        }
    }

    #[test]
    fn edits_keep_encoding_and_inserted_newline_stays_lf() {
        let path = p("w.txt");
        let mut fs = MemoryFs::new();
        fs.insert(&path, &encode("x\r\ny", Encoding::Utf16Be));
        let mut doc = Document::open(&fs, &path).unwrap();
        assert_eq!(doc.encoding(), Encoding::Utf16Be);
        assert_eq!(doc.buffer().text(), "x\r\ny");
        doc.buffer_mut().insert(4, "\né😀").unwrap();
        doc.save(&mut fs).unwrap();
        assert_eq!(
            fs.get(&path).unwrap(),
            encode("x\r\ny\né😀", Encoding::Utf16Be)
        );
    }

    #[test]
    fn dirty_follows_content() {
        let path = p("d");
        let mut fs = MemoryFs::new();
        fs.insert(&path, b"abc");
        let mut doc = Document::open(&fs, &path).unwrap();
        assert!(!doc.is_dirty());
        doc.buffer_mut().insert(1, "Z").unwrap();
        assert!(doc.is_dirty());
        doc.buffer_mut().delete(1..2).unwrap();
        assert!(!doc.is_dirty());
        // Same length, different content.
        doc.buffer_mut().delete(0..1).unwrap();
        doc.buffer_mut().insert(0, "q").unwrap();
        assert!(doc.is_dirty());
        doc.save(&mut fs).unwrap();
        assert!(!doc.is_dirty());
        assert_eq!(fs.get(&path).unwrap(), b"qbc");
        // Saving changes the reference: going back to the opened text is now dirty.
        doc.buffer_mut().delete(0..1).unwrap();
        doc.buffer_mut().insert(0, "a").unwrap();
        assert!(doc.is_dirty());
    }

    #[test]
    fn failed_save_keeps_file_and_dirty_state() {
        let path = p("s");
        let mut fs = MemoryFs::new();
        fs.insert(&path, b"orig");
        let mut doc = Document::open(&fs, &path).unwrap();
        doc.buffer_mut().insert(4, "!").unwrap();
        fs.set_fail_writes(true);
        assert!(matches!(doc.save(&mut fs), Err(VfsError::Io { .. })));
        assert!(doc.is_dirty());
        assert_eq!(fs.get(&path).unwrap(), b"orig");
        fs.set_fail_writes(false);
        assert_eq!(doc.save(&mut fs).unwrap(), 5);
        assert!(!doc.is_dirty());
        assert_eq!(fs.get(&path).unwrap(), b"orig!");
    }

    #[test]
    fn open_errors() {
        let mut fs = MemoryFs::new();
        fs.insert(&p("dir/f"), b"x");
        fs.insert(&p("bad"), b"ab\xffcd");
        assert_eq!(
            Document::open(&fs, &p("none")).unwrap_err(),
            OpenError::Vfs(VfsError::NotFound { path: p("none") })
        );
        assert_eq!(
            Document::open(&fs, &p("dir")).unwrap_err(),
            OpenError::Vfs(VfsError::NotAFile { path: p("dir") })
        );
        let err = Document::open(&fs, &p("bad")).unwrap_err();
        assert_eq!(
            err,
            OpenError::Decode {
                path: p("bad"),
                error: DecodeError {
                    encoding: Encoding::Utf8,
                    byte_offset: 2
                }
            }
        );
        assert_eq!(
            err.to_string(),
            "cannot open bad: invalid utf-8 at byte offset 2"
        );
    }

    #[test]
    fn disk_save_onto_path_replaced_by_directory_fails_and_changes_nothing() {
        use crate::disk::DiskFs;
        let root = std::env::temp_dir().join(format!(
            "kiln-workspace-unit-{}-doc-dir",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut disk = DiskFs::new(&root).unwrap();
        disk.write_atomic(&p("f.rs"), b"fn a() {}\r\n").unwrap();
        let mut doc = Document::open(&disk, &p("f.rs")).unwrap();
        doc.buffer_mut().insert(0, "//\n").unwrap();
        std::fs::remove_file(root.join("f.rs")).unwrap();
        std::fs::create_dir(root.join("f.rs")).unwrap();
        std::fs::write(root.join("f.rs/inner"), b"keep").unwrap();
        let listing = |d: &std::path::Path| {
            let mut v: Vec<_> = std::fs::read_dir(d)
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            v.sort();
            v
        };
        let before = listing(&root);
        let err = doc.save(&mut disk).unwrap_err();
        assert_eq!(err, VfsError::NotAFile { path: p("f.rs") });
        assert!(!err.to_string().contains(root.to_string_lossy().as_ref()));
        assert_eq!(listing(&root), before);
        assert_eq!(listing(&root.join("f.rs")), ["inner"]);
        assert_eq!(std::fs::read(root.join("f.rs/inner")).unwrap(), b"keep");
        assert!(doc.is_dirty());
    }

    #[test]
    fn works_through_trait_objects() {
        let mut boxed: Box<dyn Vfs> = Box::new(MemoryFs::new());
        boxed.write_atomic(&p("t"), b"hi").unwrap();
        let mut doc = Document::open(boxed.as_ref(), &p("t")).unwrap();
        assert_eq!(doc.save(boxed.as_mut()).unwrap(), 2);
    }
}
