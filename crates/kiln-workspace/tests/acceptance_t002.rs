//! Acceptance tests for T-002 (documents, file I/O through a VFS, encodings, atomic save),
//! written from the task specification alone by the test author.
//!
//! Every test drives the public API declared in the task's Interface section. Disk tests use
//! a fresh directory below `CARGO_TARGET_TMPDIR`, named after the test, so runs are
//! deterministic and independent.

use kiln_workspace::{
    DecodeError, DiskFs, Document, Encoding, MemoryFs, OpenError, RelPath, Vfs, VfsError, decode,
    encode,
};
use support::*;

#[cfg(test)]
mod support {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    pub fn rp(s: &str) -> RelPath {
        RelPath::new(s).unwrap_or_else(|e| panic!("RelPath::new({s:?}) rejected: {e:?}"))
    }

    /// A fresh, empty directory unique to `name` below the integration-test temp dir.
    pub fn fresh_dir(name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("acceptance_t002")
            .join(name);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Write a file (creating parents) directly on disk, bypassing the VFS.
    pub fn put(root: &Path, rel: &str, bytes: &[u8]) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, bytes).unwrap();
    }

    /// Sorted names of the entries directly inside `dir`.
    pub fn listing(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    /// Every regular file below `root` (relative `/`-path → bytes) and every directory.
    pub fn snapshot(root: &Path) -> (BTreeMap<String, Vec<u8>>, BTreeSet<String>) {
        fn walk(
            root: &Path,
            dir: &Path,
            files: &mut BTreeMap<String, Vec<u8>>,
            dirs: &mut BTreeSet<String>,
        ) {
            for e in std::fs::read_dir(dir).unwrap() {
                let e = e.unwrap();
                let p = e.path();
                let rel: Vec<String> = p
                    .strip_prefix(root)
                    .unwrap()
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                let rel = rel.join("/");
                let ft = e.file_type().unwrap();
                if ft.is_dir() {
                    dirs.insert(rel);
                    walk(root, &p, files, dirs);
                } else {
                    files.insert(rel, std::fs::read(&p).unwrap());
                }
            }
        }
        let mut files = BTreeMap::new();
        let mut dirs = BTreeSet::new();
        walk(root, root, &mut files, &mut dirs);
        (files, dirs)
    }

    /// Deterministic xorshift64* generator (fixed seed, no external crate).
    pub struct Rng(u64);
    impl Rng {
        pub fn new(seed: u64) -> Self {
            Rng(seed | 1)
        }
        pub fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        pub fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
        pub fn bytes(&mut self) -> Vec<u8> {
            let len = self.below(12);
            (0..len).map(|_| self.next() as u8).collect()
        }
        pub fn string(&mut self) -> String {
            const POOL: &[char] = &[
                'a',
                'Z',
                ' ',
                '\n',
                '\r',
                '\t',
                '\0',
                'é',
                'ß',
                '€',
                '✓',
                '\u{FEFF}',
                '\u{FFFF}',
                '\u{E000}',
                '\u{D7FF}',
                '😀',
                '𝄞',
                '\u{10FFFF}',
                '\u{7F}',
                '\u{80}',
            ];
            let len = self.below(16);
            (0..len)
                .map(|_| {
                    if self.below(4) == 0 {
                        // Any scalar value; surrogate code points map to a fixed char.
                        let v = (self.next() % 0x11_0000) as u32;
                        char::from_u32(v).unwrap_or('\u{FFFD}')
                    } else {
                        POOL[self.below(POOL.len())]
                    }
                })
                .collect()
        }
    }

    /// UTF-16 bytes with BOM, built with std only (independent of `encode`).
    pub fn utf16_with_bom(text: &str, little_endian: bool) -> Vec<u8> {
        let mut out = if little_endian {
            vec![0xFF, 0xFE]
        } else {
            vec![0xFE, 0xFF]
        };
        for u in text.encode_utf16() {
            if little_endian {
                out.extend_from_slice(&u.to_le_bytes());
            } else {
                out.extend_from_slice(&u.to_be_bytes());
            }
        }
        out
    }

    pub fn insert_end(doc: &mut Document, s: &str) {
        let end = doc.buffer().len_chars();
        doc.buffer_mut().insert(end, s).unwrap();
    }

    /// Permission bits and inode (Unix); `None` elsewhere.
    #[cfg(unix)]
    pub fn mode_and_inode(p: &Path) -> Option<(u32, u64)> {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let m = std::fs::metadata(p).unwrap();
        Some((m.permissions().mode() & 0o7777, m.ino()))
    }
    #[cfg(not(unix))]
    pub fn mode_and_inode(_p: &Path) -> Option<(u32, u64)> {
        None
    }

    #[cfg(unix)]
    pub fn set_mode(p: &Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)).unwrap();
    }
    #[cfg(not(unix))]
    pub fn set_mode(_p: &Path, _mode: u32) {}

    #[cfg(unix)]
    pub fn make_symlink(target: &str, link: &Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }
    #[cfg(windows)]
    pub fn make_symlink(target: &str, link: &Path) {
        std::os::windows::fs::symlink_file(target, link).unwrap();
    }

    /// The UTF-8 corpus of AC-3: name → content.
    pub fn utf8_corpus() -> Vec<(&'static str, &'static [u8])> {
        vec![
            ("empty.txt", b""),
            ("ascii.rs", b"fn main() {}\n"),
            (
                "multibyte.txt",
                "h\u{e9}llo w\u{f6}rld \u{2713} \u{20ac} \u{1F600}\u{1D11E}\n".as_bytes(),
            ),
            ("lf.txt", b"one\ntwo\nthree\n"),
            ("crlf.txt", b"one\r\ntwo\r\nthree\r\n"),
            ("cr.txt", b"one\rtwo\rthree\r"),
            ("mixed.txt", b"a\nb\r\nc\rd\r\n\r\n\n\r"),
            ("no_final_newline.txt", b"last line has no newline"),
            ("nul.bin", b"nul\0inside\0\0end\n"),
            ("only_newlines.txt", b"\r\n\n\r\r\n"),
            (
                "dir/nested/deep.rs",
                "// \u{1F980} crab\r\nlet x = 1;".as_bytes(),
            ),
        ]
    }
}

// ---------------------------------------------------------------- AC-1 path validation

#[test]
fn relpath_accepts_and_rejects_exact_cases() {
    for ok in [
        "a",
        "a/b.rs",
        "src/main.rs",
        "a b/c",
        "a.b",
        "...",
        ".hidden/x",
        "é/😀",
    ] {
        let p = RelPath::new(ok).unwrap_or_else(|e| panic!("{ok:?} must be accepted: {e:?}"));
        assert_eq!(p.as_str(), ok);
        assert_eq!(p.to_string(), ok);
        assert_eq!(p, RelPath::new(ok).unwrap());
    }
    for bad in [
        "", "/a", "a/", "a//b", "./a", "a/../b", "..", "a\\b", "a\0b", ".", "a/.", "a/./b", "../a",
        "/", "//", "\\", "\0", "a/..",
    ] {
        let r = RelPath::new(bad);
        assert!(r.is_err(), "{bad:?} must be rejected, got {r:?}");
        let e = r.unwrap_err();
        assert!(!e.to_string().is_empty(), "PathError Display is empty");
    }
}

// ---------------------------------------------------------------- AC-2 VFS differential

/// Apply the same fixed-seed sequence to MemoryFs and DiskFs (fresh dir) and compare every
/// result, plus a third model (map of last bytes written) for read∘write = identity.
#[test]
fn memory_and_disk_vfs_agree_on_random_operation_sequences() {
    let root = fresh_dir("vfs_differential");
    let initial: &[(&str, &[u8])] = &[("a", b"alpha"), ("d/x", b"x0"), ("d/e/z", b"")];
    let mut mem = MemoryFs::new();
    for (p, b) in initial {
        mem.insert(&rp(p), b);
        put(&root, p, b);
    }
    let mut disk = DiskFs::new(&root).unwrap();
    let mut model: std::collections::BTreeMap<String, Vec<u8>> = initial
        .iter()
        .map(|(p, b)| (p.to_string(), b.to_vec()))
        .collect();

    // Files, nested files, new files in existing dirs, missing files, missing parent
    // directories, and paths that are directories.
    let pool = [
        "a",
        "b.txt",
        "d/x",
        "d/y",
        "d/e/z",
        "d/e/w",
        "missing",
        "nodir/f",
        "d/nodir/f",
        "d",
        "d/e",
    ];
    let dirs = ["d", "d/e"];
    let missing_parent = ["nodir/f", "d/nodir/f"];

    let mut rng = Rng::new(0x7002_5EED);
    for step in 0..600 {
        let path = pool[rng.below(pool.len())];
        let p = rp(path);
        match rng.below(3) {
            0 => {
                let bytes = rng.bytes();
                let m = mem.write_atomic(&p, &bytes);
                let d = disk.write_atomic(&p, &bytes);
                assert_eq!(m, d, "step {step}: write_atomic({path}) differs");
                if dirs.contains(&path) {
                    assert_eq!(
                        m,
                        Err(VfsError::NotAFile { path: p.clone() }),
                        "step {step}"
                    );
                } else if missing_parent.contains(&path) {
                    assert!(
                        matches!(m, Err(VfsError::NotFound { .. })),
                        "step {step}: {m:?}"
                    );
                } else {
                    assert_eq!(m, Ok(()), "step {step}");
                    model.insert(path.to_string(), bytes);
                }
            }
            1 => {
                let m = mem.read(&p);
                let d = disk.read(&p);
                assert_eq!(m, d, "step {step}: read({path}) differs");
                match model.get(path) {
                    Some(b) => assert_eq!(m, Ok(b.clone()), "step {step}: read({path})"),
                    None if dirs.contains(&path) => {
                        assert_eq!(
                            m,
                            Err(VfsError::NotAFile { path: p.clone() }),
                            "step {step}"
                        )
                    }
                    None => {
                        assert_eq!(
                            m,
                            Err(VfsError::NotFound { path: p.clone() }),
                            "step {step}"
                        )
                    }
                }
            }
            _ => {
                let m = mem.is_file(&p);
                let d = disk.is_file(&p);
                assert_eq!(m, d, "step {step}: is_file({path}) differs");
                assert_eq!(m, model.contains_key(path), "step {step}: is_file({path})");
            }
        }
    }

    // Final contents: identical on both, equal to the model, no directory created.
    let mem_files: std::collections::BTreeMap<String, Vec<u8>> = mem
        .paths()
        .into_iter()
        .map(|p| {
            let b = mem.get(&p).unwrap();
            (p.as_str().to_string(), b)
        })
        .collect();
    let (disk_files, disk_dirs) = snapshot(&root);
    assert_eq!(mem_files, model);
    assert_eq!(disk_files, model);
    let expected_dirs: std::collections::BTreeSet<String> =
        dirs.iter().map(|s| s.to_string()).collect();
    assert_eq!(disk_dirs, expected_dirs, "no directory may be created");
    let mut sorted = mem.paths();
    sorted.sort();
    assert_eq!(mem.paths(), sorted, "MemoryFs::paths must be sorted");
}

// ---------------------------------------------------------------- AC-3 UTF-8 round trip

#[test]
fn unedited_utf8_files_round_trip_byte_for_byte() {
    let corpus = utf8_corpus();
    let root = fresh_dir("utf8_round_trip");
    let mut mem = MemoryFs::new();
    for (p, b) in &corpus {
        mem.insert(&rp(p), b);
        put(&root, p, b);
    }
    let mut disk = DiskFs::new(&root).unwrap();
    for (p, original) in &corpus {
        let path = rp(p);
        let text = std::str::from_utf8(original).unwrap();
        for vfs in [&mut mem as &mut dyn Vfs, &mut disk as &mut dyn Vfs] {
            let mut doc = Document::open(&*vfs, &path).unwrap();
            assert_eq!(doc.encoding(), Encoding::Utf8, "{p}");
            assert_eq!(doc.path(), &path);
            assert_eq!(doc.buffer().text(), text, "{p}: buffer text");
            assert!(!doc.is_dirty(), "{p}");
            let written = doc.save(vfs).unwrap();
            assert_eq!(written, original.len(), "{p}: bytes written");
            assert_eq!(
                vfs.read(&path).unwrap(),
                original.to_vec(),
                "{p}: saved bytes"
            );
        }
        assert_eq!(
            std::fs::read(root.join(p)).unwrap(),
            original.to_vec(),
            "{p} on disk"
        );
    }
}

#[test]
fn line_endings_round_trip_verbatim() {
    let mut mem = MemoryFs::new();
    let path = rp("mixed.txt");
    let original: &[u8] = b"a\r\nb\nc\rd\r\n";
    mem.insert(&path, original);

    let mut doc = Document::open(&mem, &path).unwrap();
    // The buffer holds exactly the file's \n, \r\n and \r characters.
    assert_eq!(doc.buffer().text(), "a\r\nb\nc\rd\r\n");
    assert_eq!(doc.buffer().len_chars(), 10);
    doc.save(&mut mem).unwrap();
    assert_eq!(mem.get(&path).unwrap(), original.to_vec());

    // An inserted "\n" stays "\n" even in a CRLF-dominated file; nothing is converted.
    let crlf = rp("crlf.txt");
    mem.insert(&crlf, b"x\r\ny\r\n");
    let mut doc = Document::open(&mem, &crlf).unwrap();
    doc.buffer_mut().insert(3, "new\n").unwrap();
    insert_end(&mut doc, "\rtail");
    doc.save(&mut mem).unwrap();
    assert_eq!(mem.get(&crlf).unwrap(), b"x\r\nnew\ny\r\n\rtail".to_vec());

    // A lone CR at the very end and a CR before LF that is split by an edit stay as they are.
    let cr = rp("cr.txt");
    mem.insert(&cr, b"p\r");
    let mut doc = Document::open(&mem, &cr).unwrap();
    insert_end(&mut doc, "\nq");
    doc.save(&mut mem).unwrap();
    assert_eq!(mem.get(&cr).unwrap(), b"p\r\nq".to_vec());
    let reopened = Document::open(&mem, &cr).unwrap();
    assert_eq!(reopened.buffer().text(), "p\r\nq");
}

// ---------------------------------------------------------------- AC-4 BOM

#[test]
fn utf8_bom_detected_stripped_and_restored_on_save() {
    let root = fresh_dir("utf8_bom");
    let original: &[u8] = b"\xEF\xBB\xBFfn x() {}\r\n\xC3\xA9";
    put(&root, "bom.rs", original);
    let mut mem = MemoryFs::new();
    let path = rp("bom.rs");
    mem.insert(&path, original);
    let mut disk = DiskFs::new(&root).unwrap();

    for vfs in [&mut mem as &mut dyn Vfs, &mut disk as &mut dyn Vfs] {
        let mut doc = Document::open(&*vfs, &path).unwrap();
        assert_eq!(doc.encoding(), Encoding::Utf8Bom);
        assert_eq!(doc.buffer().text(), "fn x() {}\r\n\u{e9}");
        assert!(!doc.buffer().text().starts_with('\u{FEFF}'));
        // Unedited save: identical bytes.
        assert_eq!(doc.save(vfs).unwrap(), original.len());
        assert_eq!(vfs.read(&path).unwrap(), original.to_vec());
        // Edited save: BOM followed by the UTF-8 text.
        doc.buffer_mut().insert(0, "// \u{1F600}\n").unwrap();
        doc.save(vfs).unwrap();
        let mut expected = vec![0xEF, 0xBB, 0xBF];
        expected.extend_from_slice("// \u{1F600}\nfn x() {}\r\n\u{e9}".as_bytes());
        assert_eq!(vfs.read(&path).unwrap(), expected);
        assert_eq!(doc.encoding(), Encoding::Utf8Bom);
        // Reopening gives the edited text, still with BOM detected.
        let again = Document::open(&*vfs, &path).unwrap();
        assert_eq!(again.encoding(), Encoding::Utf8Bom);
        assert_eq!(again.buffer().text(), "// \u{1F600}\nfn x() {}\r\n\u{e9}");
    }

    // Only the first BOM is a BOM: a second U+FEFF is text and survives the round trip.
    let double = rp("double.txt");
    let bytes: &[u8] = b"\xEF\xBB\xBF\xEF\xBB\xBFa";
    mem.insert(&double, bytes);
    let mut doc = Document::open(&mem, &double).unwrap();
    assert_eq!(doc.encoding(), Encoding::Utf8Bom);
    assert_eq!(doc.buffer().text(), "\u{FEFF}a");
    doc.save(&mut mem).unwrap();
    assert_eq!(mem.get(&double).unwrap(), bytes.to_vec());

    assert_eq!(
        decode(original),
        Ok(("fn x() {}\r\n\u{e9}".to_string(), Encoding::Utf8Bom))
    );
}

#[test]
fn bom_only_file_round_trips() {
    let root = fresh_dir("bom_only");
    put(&root, "b.txt", b"\xEF\xBB\xBF");
    let mut mem = MemoryFs::new();
    let path = rp("b.txt");
    mem.insert(&path, b"\xEF\xBB\xBF");
    let mut disk = DiskFs::new(&root).unwrap();
    for vfs in [&mut mem as &mut dyn Vfs, &mut disk as &mut dyn Vfs] {
        let mut doc = Document::open(&*vfs, &path).unwrap();
        assert_eq!(doc.encoding(), Encoding::Utf8Bom);
        assert_eq!(doc.buffer().text(), "");
        assert!(doc.buffer().is_empty());
        assert_eq!(doc.save(vfs).unwrap(), 3);
        assert_eq!(vfs.read(&path).unwrap(), vec![0xEF, 0xBB, 0xBF]);
    }
    assert_eq!(
        decode(b"\xEF\xBB\xBF"),
        Ok((String::new(), Encoding::Utf8Bom))
    );
    assert_eq!(encode("", Encoding::Utf8Bom), vec![0xEF, 0xBB, 0xBF]);
}

// ---------------------------------------------------------------- AC-5 UTF-16

#[test]
fn utf16_le_and_be_with_bom_open_and_save() {
    // "hi\r\n" U+1D11E "x" — written out by hand, including the surrogate pair D834 DD1E.
    let expected_text = "hi\r\n\u{1D11E}x";
    let le: Vec<u8> = vec![
        0xFF, 0xFE, 0x68, 0x00, 0x69, 0x00, 0x0D, 0x00, 0x0A, 0x00, 0x34, 0xD8, 0x1E, 0xDD, 0x78,
        0x00,
    ];
    let be: Vec<u8> = vec![
        0xFE, 0xFF, 0x00, 0x68, 0x00, 0x69, 0x00, 0x0D, 0x00, 0x0A, 0xD8, 0x34, 0xDD, 0x1E, 0x00,
        0x78,
    ];
    assert_eq!(
        utf16_with_bom(expected_text, true),
        le,
        "test fixture self-check"
    );
    assert_eq!(
        utf16_with_bom(expected_text, false),
        be,
        "test fixture self-check"
    );

    let root = fresh_dir("utf16");
    put(&root, "le.txt", &le);
    put(&root, "be.txt", &be);
    let mut mem = MemoryFs::new();
    mem.insert(&rp("le.txt"), &le);
    mem.insert(&rp("be.txt"), &be);
    let mut disk = DiskFs::new(&root).unwrap();

    for vfs in [&mut mem as &mut dyn Vfs, &mut disk as &mut dyn Vfs] {
        for (name, bytes, enc, little) in [
            ("le.txt", &le, Encoding::Utf16Le, true),
            ("be.txt", &be, Encoding::Utf16Be, false),
        ] {
            let path = rp(name);
            let mut doc = Document::open(&*vfs, &path).unwrap();
            assert_eq!(doc.encoding(), enc, "{name}");
            assert_eq!(doc.buffer().text(), expected_text, "{name}");
            assert_eq!(doc.buffer().len_chars(), 6, "{name}");
            assert_eq!(doc.save(vfs).unwrap(), bytes.len(), "{name}");
            assert_eq!(&vfs.read(&path).unwrap(), bytes, "{name}: unedited save");

            doc.buffer_mut().insert(2, "\u{e9}\u{1F600}").unwrap();
            let edited = "hi\u{e9}\u{1F600}\r\n\u{1D11E}x";
            assert_eq!(doc.buffer().text(), edited);
            let n = doc.save(vfs).unwrap();
            let independent = utf16_with_bom(edited, little);
            assert_eq!(vfs.read(&path).unwrap(), independent, "{name}: edited save");
            assert_eq!(n, independent.len());
            assert_eq!(encode(edited, enc), independent, "{name}: encode");
            assert_eq!(doc.encoding(), enc);
        }
    }

    assert_eq!(
        decode(&le),
        Ok((expected_text.to_string(), Encoding::Utf16Le))
    );
    assert_eq!(
        decode(&be),
        Ok((expected_text.to_string(), Encoding::Utf16Be))
    );
    assert_eq!(
        decode(&[0xFF, 0xFE]),
        Ok((String::new(), Encoding::Utf16Le))
    );
    assert_eq!(
        decode(&[0xFE, 0xFF]),
        Ok((String::new(), Encoding::Utf16Be))
    );
}

// ---------------------------------------------------------------- AC-6 invalid input

#[test]
fn invalid_bytes_are_reported_with_offset_never_replaced() {
    use Encoding::*;
    let cases: &[(&[u8], Encoding, usize)] = &[
        // The six cases named by AC-6.
        (b"ab\xffcd", Utf8, 2),
        (b"\xEF\xBB\xBF\x61\xC3", Utf8Bom, 4),
        (b"a\xe2\x82", Utf8, 1),
        (b"\xFF\xFE\x61\x00\x62", Utf16Le, 4),
        (b"\xFF\xFE\x00\xD8\x61\x00", Utf16Le, 2),
        (b"\xFE\xFF\xDC\x00", Utf16Be, 2),
        // Further hostile inputs implied by the rule "first byte of the first invalid sequence".
        (b"\xff", Utf8, 0),
        (b"\xfe", Utf8, 0),
        (b"\xEF\xBB", Utf8, 0),
        (b"\xc3\x28", Utf8, 0),
        (b"\xc0\xaf", Utf8, 0),
        (b"ok\xed\xa0\x80", Utf8, 2),
        (b"\xf4\x90\x80\x80", Utf8, 0),
        (b"x\xe2\x82\xacy\x80", Utf8, 5),
        (b"\xEF\xBB\xBF\xff", Utf8Bom, 3),
        (b"\xFE\xFF\x00\x61\x00", Utf16Be, 4),
        (b"\xFF\xFE\x3D\xD8", Utf16Le, 2),
        (b"\xFF\xFE\x61\x00\x00\xDC", Utf16Le, 4),
        (b"\xFE\xFF\xD8\x3D\x00\x61", Utf16Be, 2),
        (b"\xFF\xFE\x3D\xD8\x3D\xD8\x00\xDE", Utf16Le, 2),
    ];
    let mut mem = MemoryFs::new();
    for (i, (bytes, enc, off)) in cases.iter().enumerate() {
        let want = DecodeError {
            encoding: *enc,
            byte_offset: *off,
        };
        assert_eq!(decode(bytes), Err(want.clone()), "case {i}: {bytes:02X?}");
        assert!(!want.to_string().is_empty());

        let path = rp(&format!("bad/{i}.txt"));
        mem.insert(&path, bytes);
        match Document::open(&mem, &path) {
            Err(OpenError::Decode { path: p, error }) => {
                assert_eq!(p, path, "case {i}");
                assert_eq!(error, want, "case {i}");
            }
            other => panic!("case {i}: expected OpenError::Decode, got {other:?}"),
        }
        assert_eq!(
            mem.get(&path).unwrap(),
            bytes.to_vec(),
            "case {i}: file untouched"
        );
    }

    // The same on disk.
    let root = fresh_dir("invalid_bytes");
    put(&root, "bad.txt", b"ab\xffcd");
    let disk = DiskFs::new(&root).unwrap();
    let path = rp("bad.txt");
    assert_eq!(
        Document::open(&disk, &path).map(|_| ()),
        Err(OpenError::Decode {
            path: path.clone(),
            error: DecodeError {
                encoding: Utf8,
                byte_offset: 2
            }
        })
    );
    assert_eq!(
        std::fs::read(root.join("bad.txt")).unwrap(),
        b"ab\xffcd".to_vec()
    );
}

// ---------------------------------------------------------------- AC-7 encode/decode inverse

#[test]
fn encode_decode_round_trip() {
    use Encoding::*;
    assert_eq!(Utf8.name(), "utf-8");
    assert_eq!(Utf8Bom.name(), "utf-8-bom");
    assert_eq!(Utf16Le.name(), "utf-16le");
    assert_eq!(Utf16Be.name(), "utf-16be");

    let mut strings: Vec<String> = vec![
        String::new(),
        "\u{FEFF}".to_string(),
        "\u{FEFF}\u{FEFF}x".to_string(),
        "\u{FEFF}abc".to_string(),
        "a\u{FEFF}".to_string(),
        "\0".to_string(),
        "\r\n\r".to_string(),
        "\u{10FFFF}\u{10000}\u{FFFF}\u{D7FF}\u{E000}".to_string(),
    ];
    let mut rng = Rng::new(0xE7C0_DE02);
    for _ in 0..2000 {
        strings.push(rng.string());
    }
    for s in &strings {
        for e in [Utf8, Utf8Bom, Utf16Le, Utf16Be] {
            let bytes = encode(s, e);
            let got = decode(&bytes);
            if e == Utf8 && s.starts_with('\u{FEFF}') {
                // Documented ambiguity: a leading U+FEFF in plain UTF-8 reads as a BOM.
                let rest = &s['\u{FEFF}'.len_utf8()..];
                assert_eq!(got, Ok((rest.to_string(), Utf8Bom)), "{s:?} as {e:?}");
            } else {
                assert_eq!(got, Ok((s.clone(), e)), "{s:?} as {e:?}");
            }
            // encode is BOM + the text in that encoding (exact value).
            let expected = match e {
                Utf8 => s.as_bytes().to_vec(),
                Utf8Bom => [&[0xEF, 0xBB, 0xBF][..], s.as_bytes()].concat(),
                Utf16Le => utf16_with_bom(s, true),
                Utf16Be => utf16_with_bom(s, false),
            };
            assert_eq!(bytes, expected, "encode({s:?}, {e:?})");
        }
    }
}

// ---------------------------------------------------------------- AC-8 dirty

#[test]
fn dirty_follows_text_versus_last_saved() {
    let mut mem = MemoryFs::new();
    let path = rp("d.txt");
    mem.insert(&path, b"abc\n");
    let mut doc = Document::open(&mem, &path).unwrap();
    assert!(!doc.is_dirty(), "fresh document is clean");

    doc.buffer_mut().insert(1, "Z").unwrap();
    assert!(doc.is_dirty(), "after insert");
    doc.buffer_mut().delete(1..2).unwrap();
    assert!(!doc.is_dirty(), "insert then inverse delete");

    // Content-based (D-1): delete then re-insert the same text is clean.
    let removed = doc.buffer_mut().delete(0..2).unwrap();
    assert_eq!(removed, "ab");
    assert!(doc.is_dirty());
    doc.buffer_mut().insert(0, "ab").unwrap();
    assert!(!doc.is_dirty(), "delete then re-insert");

    // Save: clean, and the file holds encode(text, encoding).
    doc.buffer_mut().insert(4, "more").unwrap();
    assert!(doc.is_dirty());
    let n = doc.save(&mut mem).unwrap();
    assert!(!doc.is_dirty(), "after save");
    let expected = encode(&doc.buffer().text(), doc.encoding());
    assert_eq!(expected, b"abc\nmore".to_vec());
    assert_eq!(mem.read(&path).unwrap(), expected);
    assert_eq!(n, expected.len());

    // The reference is now the saved text, not the opened text.
    doc.buffer_mut().delete(4..8).unwrap();
    assert_eq!(doc.buffer().text(), "abc\n");
    assert!(
        doc.is_dirty(),
        "back to opened text but differs from last saved"
    );
    doc.buffer_mut().insert(4, "more").unwrap();
    assert!(!doc.is_dirty(), "back to last saved text");

    // Save writes even when clean.
    mem.insert(&path, b"changed behind our back");
    assert!(!doc.is_dirty());
    doc.save(&mut mem).unwrap();
    assert_eq!(mem.get(&path).unwrap(), b"abc\nmore".to_vec());

    // Naive model over a deterministic edit sequence, on a UTF-16 document.
    let p16 = rp("u16.txt");
    mem.insert(&p16, &encode("x\u{e9}\r\n", Encoding::Utf16Be));
    let mut doc = Document::open(&mem, &p16).unwrap();
    let mut last_saved = doc.buffer().text();
    let mut rng = Rng::new(0xD1A7);
    for step in 0..400 {
        let len = doc.buffer().len_chars();
        match rng.below(4) {
            0 => {
                let at = rng.below(len + 1);
                let s = ["a", "\u{e9}", "\u{1F600}", "\r\n", ""][rng.below(5)];
                doc.buffer_mut().insert(at, s).unwrap();
            }
            1 if len > 0 => {
                let a = rng.below(len + 1);
                let b = a + rng.below(len - a + 1);
                doc.buffer_mut().delete(a..b).unwrap();
            }
            2 => {
                doc.save(&mut mem).unwrap();
                last_saved = doc.buffer().text();
                assert_eq!(
                    mem.get(&p16).unwrap(),
                    encode(&last_saved, Encoding::Utf16Be),
                    "step {step}"
                );
            }
            _ => {
                // Restore the last saved text by edits.
                let n = doc.buffer().len_chars();
                doc.buffer_mut().delete(0..n).unwrap();
                doc.buffer_mut().insert(0, &last_saved).unwrap();
            }
        }
        assert_eq!(
            doc.is_dirty(),
            doc.buffer().text() != last_saved,
            "step {step}"
        );
    }
}

// ---------------------------------------------------------------- AC-9 failed save

#[test]
fn failed_save_leaves_file_intact_and_document_dirty() {
    // MemoryFs with injected write failure.
    let mut mem = MemoryFs::new();
    let path = rp("src/lib.rs");
    let original: &[u8] = b"pub fn f() {}\n";
    mem.insert(&path, original);
    mem.insert(&rp("other.txt"), b"o");
    let mut doc = Document::open(&mem, &path).unwrap();
    doc.buffer_mut().insert(0, "// edit\n").unwrap();
    let before = mem.clone();

    mem.set_fail_writes(true);
    match doc.save(&mut mem) {
        Err(VfsError::Io { path: p, kind, .. }) => {
            assert_eq!(p, path);
            assert_eq!(kind, std::io::ErrorKind::Other);
        }
        other => panic!("expected VfsError::Io, got {other:?}"),
    }
    assert_eq!(
        mem.get(&path).unwrap(),
        original.to_vec(),
        "file keeps original bytes"
    );
    assert!(doc.is_dirty(), "still dirty after failed save");
    assert_eq!(doc.buffer().text(), "// edit\npub fn f() {}\n");
    // A write of a new file while failing creates nothing.
    let newp = rp("src/new.rs");
    assert!(matches!(
        mem.write_atomic(&newp, b"n"),
        Err(VfsError::Io { .. })
    ));
    assert_eq!(mem.get(&newp), None);
    let mut after = mem.clone();
    after.set_fail_writes(false);
    let mut before_cmp = before.clone();
    before_cmp.set_fail_writes(false);
    assert_eq!(after, before_cmp, "nothing changed in the MemoryFs");
    assert_eq!(mem.paths(), before.paths());

    mem.set_fail_writes(false);
    let n = doc.save(&mut mem).unwrap();
    assert!(!doc.is_dirty());
    let want = b"// edit\npub fn f() {}\n".to_vec();
    assert_eq!(n, want.len());
    assert_eq!(mem.get(&path).unwrap(), want);

    // DiskFs: the document's path was replaced by a directory since open.
    let root = fresh_dir("failed_save_disk");
    put(&root, "d/f.txt", b"text\n");
    put(&root, "d/sibling.txt", b"s");
    let mut disk = DiskFs::new(&root).unwrap();
    let fpath = rp("d/f.txt");
    let mut doc = Document::open(&disk, &fpath).unwrap();
    doc.buffer_mut().insert(0, "edited ").unwrap();
    std::fs::remove_file(root.join("d/f.txt")).unwrap();
    put(&root, "d/f.txt/inner.txt", b"inner");
    let parent_before = listing(&root.join("d"));
    let snap_before = snapshot(&root);

    let r = doc.save(&mut disk);
    assert!(r.is_err(), "save onto a directory must fail, got {r:?}");
    assert!(root.join("d/f.txt").is_dir());
    assert_eq!(
        listing(&root.join("d")),
        parent_before,
        "no entry appeared in the parent"
    );
    assert_eq!(
        snapshot(&root),
        snap_before,
        "directory and its contents unchanged"
    );
    assert!(doc.is_dirty());
    assert_eq!(doc.buffer().text(), "edited text\n");
}

// ---------------------------------------------------------------- AC-10 atomic replace

#[test]
fn disk_save_replaces_via_rename_without_leftovers() {
    let root = fresh_dir("atomic_replace");
    put(&root, "d/f.txt", b"old content\n");
    put(&root, "d/keep.txt", b"k");
    let file = root.join("d/f.txt");
    set_mode(&file, 0o640);
    let listing_before = listing(&root.join("d"));
    let root_before = listing(&root);
    let meta_before = mode_and_inode(&file);

    let mut disk = DiskFs::new(&root).unwrap();
    let path = rp("d/f.txt");
    let mut doc = Document::open(&disk, &path).unwrap();
    doc.buffer_mut().insert(0, "new ").unwrap();
    let n = doc.save(&mut disk).unwrap();
    assert_eq!(n, b"new old content\n".len());

    assert_eq!(
        listing(&root.join("d")),
        listing_before,
        "no temporary file left"
    );
    assert_eq!(listing(&root), root_before);
    assert_eq!(std::fs::read(&file).unwrap(), b"new old content\n".to_vec());
    assert_eq!(
        std::fs::read(root.join("d/keep.txt")).unwrap(),
        b"k".to_vec()
    );
    let meta_after = mode_and_inode(&file);
    if let (Some((mode_b, ino_b)), Some((mode_a, ino_a))) = (meta_before, meta_after) {
        assert_eq!(mode_b, 0o640, "fixture self-check");
        assert_eq!(mode_a, mode_b, "permission bits unchanged");
        assert_ne!(
            ino_a, ino_b,
            "content swapped in by rename, not rewritten in place"
        );
    }

    // A second save (unedited) behaves the same way.
    let meta_mid = mode_and_inode(&file);
    doc.save(&mut disk).unwrap();
    assert_eq!(listing(&root.join("d")), listing_before);
    assert_eq!(std::fs::read(&file).unwrap(), b"new old content\n".to_vec());
    if let (Some((mode_m, ino_m)), Some((mode_a, ino_a))) = (meta_mid, mode_and_inode(&file)) {
        assert_eq!(mode_a, mode_m);
        assert_ne!(ino_a, ino_m);
    }

    // write_atomic of a new file in an existing directory also leaves only that file.
    disk.write_atomic(&rp("d/created.txt"), b"c").unwrap();
    let mut want = listing_before.clone();
    want.push("created.txt".to_string());
    want.sort();
    assert_eq!(listing(&root.join("d")), want);
}

#[test]
fn save_through_symlink_updates_target() {
    let root = fresh_dir("symlink_save");
    put(&root, "d/target.txt", b"target\n");
    make_symlink("target.txt", &root.join("d/link.txt"));
    let listing_before = listing(&root.join("d"));

    let mut disk = DiskFs::new(&root).unwrap();
    let path = rp("d/link.txt");
    let mut doc = Document::open(&disk, &path).unwrap();
    assert_eq!(doc.buffer().text(), "target\n");
    insert_end(&mut doc, "via link\n");
    doc.save(&mut disk).unwrap();

    let link = root.join("d/link.txt");
    let lm = std::fs::symlink_metadata(&link).unwrap();
    assert!(lm.file_type().is_symlink(), "the symlink stays a symlink");
    assert_eq!(
        std::fs::read_link(&link).unwrap(),
        std::path::PathBuf::from("target.txt")
    );
    assert_eq!(
        std::fs::read(root.join("d/target.txt")).unwrap(),
        b"target\nvia link\n".to_vec(),
        "the target file holds the new content"
    );
    assert_eq!(listing(&root.join("d")), listing_before, "no leftovers");
    assert_eq!(disk.read(&path).unwrap(), b"target\nvia link\n".to_vec());
}

// ---------------------------------------------------------------- AC-11 errors

#[test]
fn vfs_errors_name_relative_paths_only() {
    let root = fresh_dir("errors_relative");
    put(&root, "dir/inside.txt", b"i");
    let root_str = root.to_string_lossy().into_owned();
    let mut disk = DiskFs::new(&root).unwrap();
    let mut mem = MemoryFs::new();
    mem.insert(&rp("dir/inside.txt"), b"i");

    let missing = rp("missing.rs");
    let dir = rp("dir");
    let orphan = rp("nodir/new.txt");
    for (label, vfs) in [
        ("memory", &mut mem as &mut dyn Vfs),
        ("disk", &mut disk as &mut dyn Vfs),
    ] {
        let e = Document::open(&*vfs, &missing).unwrap_err();
        assert_eq!(
            e,
            OpenError::Vfs(VfsError::NotFound {
                path: missing.clone()
            }),
            "{label}"
        );
        let shown = e.to_string();
        assert!(shown.contains("missing.rs"), "{label}: {shown}");
        assert!(!shown.contains(&root_str), "{label}: {shown}");

        let e = Document::open(&*vfs, &dir).unwrap_err();
        assert_eq!(
            e,
            OpenError::Vfs(VfsError::NotAFile { path: dir.clone() }),
            "{label}"
        );
        assert!(e.to_string().contains("dir"), "{label}");
        assert!(!e.to_string().contains(&root_str), "{label}");

        let w = vfs.write_atomic(&orphan, b"x").unwrap_err();
        assert!(matches!(w, VfsError::NotFound { .. }), "{label}: {w:?}");
        let shown = w.to_string();
        assert!(shown.contains("nodir"), "{label}: {shown}");
        assert!(!shown.contains(&root_str), "{label}: {shown}");
        assert!(!vfs.is_file(&orphan), "{label}");

        let r = vfs.read(&missing).unwrap_err();
        assert!(r.to_string().contains("missing.rs"));
        assert!(!r.to_string().contains(&root_str));
    }
    // Nothing was created by the failed write.
    assert!(!root.join("nodir").exists());
    assert_eq!(listing(&root), vec!["dir".to_string()]);
    assert_eq!(mem.paths(), vec![rp("dir/inside.txt")]);

    // An Io error on disk (save onto a directory) also names only the relative path.
    put(&root, "d2/f.txt", b"f");
    let fpath = rp("d2/f.txt");
    let mut doc = Document::open(&disk, &fpath).unwrap();
    std::fs::remove_file(root.join("d2/f.txt")).unwrap();
    std::fs::create_dir(root.join("d2/f.txt")).unwrap();
    insert_end(&mut doc, "!");
    let e = doc.save(&mut disk).unwrap_err();
    let shown = e.to_string();
    assert!(shown.contains("d2/f.txt"), "{shown}");
    assert!(!shown.contains(&root_str), "{shown}");

    // Decode errors name the relative path too.
    put(&root, "bad.txt", b"\xff");
    let e = Document::open(&disk, &rp("bad.txt")).unwrap_err();
    assert!(e.to_string().contains("bad.txt"));
    assert!(!e.to_string().contains(&root_str));

    // DiskFs::new refuses a root that is not a directory.
    assert!(DiskFs::new(&root.join("bad.txt")).is_err());
    assert!(DiskFs::new(&root.join("does-not-exist")).is_err());
}

// ---------------------------------------------------------------- AC-12 determinism

#[test]
fn disk_vfs_results_do_not_depend_on_root() {
    fn run(root: &std::path::Path) -> Vec<String> {
        put(root, "a.txt", b"a");
        put(root, "d/x.txt", "\u{e9}".as_bytes());
        put(root, "d/bad.txt", b"\xEF\xBB\xBFok\xC3");
        put(root, "d/gone.txt", b"g");
        let mut fs = DiskFs::new(root).unwrap();
        let mut out = Vec::new();
        let mut rec = |r: String| out.push(r);
        let pool = ["a.txt", "b.txt", "d/x.txt", "d", "nodir/y", "d/nodir/z"];
        let mut rng = Rng::new(0xDE7E_2A11);
        for _ in 0..200 {
            let p = rp(pool[rng.below(pool.len())]);
            match rng.below(3) {
                0 => {
                    let b = rng.bytes();
                    let r = fs.write_atomic(&p, &b);
                    rec(format!("write {p}: {r:?} / {}", show(&r)));
                }
                1 => {
                    let r = fs.read(&p);
                    rec(format!("read {p}: {r:?} / {}", show(&r)));
                }
                _ => rec(format!("is_file {p}: {}", fs.is_file(&p))),
            }
        }
        for name in ["missing", "d", "d/bad.txt", "d/x.txt"] {
            let r = Document::open(&fs, &rp(name)).map(|d| d.buffer().text());
            let shown = match &r {
                Ok(t) => t.clone(),
                Err(e) => e.to_string(),
            };
            rec(format!("open {name}: {r:?} / {shown}"));
        }
        // An Io-class failure: the document's path became a directory.
        let mut doc = Document::open(&fs, &rp("d/gone.txt")).unwrap();
        std::fs::remove_file(root.join("d/gone.txt")).unwrap();
        std::fs::create_dir(root.join("d/gone.txt")).unwrap();
        insert_end(&mut doc, "!");
        let r = doc.save(&mut fs);
        rec(format!("save d/gone.txt: {r:?} / {}", show(&r)));
        out
    }
    fn show<T>(r: &Result<T, VfsError>) -> String {
        match r {
            Ok(_) => "ok".to_string(),
            Err(e) => e.to_string(),
        }
    }
    let a = run(&fresh_dir("determinism_root_one"));
    let b = run(&fresh_dir("determinism_second_root_with_a_longer_name"));
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x, y);
    }
    assert!(a.iter().any(|l| l.contains("NotFound")));
    assert!(a.iter().any(|l| l.contains("NotAFile")));
}
