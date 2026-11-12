//! Encoding detection, strict decoding and encoding.

use std::fmt;

/// A supported text file encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Encoding {
    /// UTF-8 without a byte-order mark.
    Utf8,
    /// UTF-8 preceded by the byte-order mark `EF BB BF`.
    Utf8Bom,
    /// UTF-16 little-endian preceded by the byte-order mark `FF FE`.
    Utf16Le,
    /// UTF-16 big-endian preceded by the byte-order mark `FE FF`.
    Utf16Be,
}

impl Encoding {
    /// "utf-8", "utf-8-bom", "utf-16le", "utf-16be".
    pub fn name(self) -> &'static str {
        match self {
            Encoding::Utf8 => "utf-8",
            Encoding::Utf8Bom => "utf-8-bom",
            Encoding::Utf16Le => "utf-16le",
            Encoding::Utf16Be => "utf-16be",
        }
    }

    fn bom(self) -> &'static [u8] {
        match self {
            Encoding::Utf8 => &[],
            Encoding::Utf8Bom => &[0xEF, 0xBB, 0xBF],
            Encoding::Utf16Le => &[0xFF, 0xFE],
            Encoding::Utf16Be => &[0xFE, 0xFF],
        }
    }
}

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Input that is not valid in its (detected) encoding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeError {
    /// The detected encoding.
    pub encoding: Encoding,
    /// Offset in the whole input (BOM included) of the first invalid byte; see [`decode`].
    pub byte_offset: usize,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid {} at byte offset {}",
            self.encoding.name(),
            self.byte_offset
        )
    }
}

impl std::error::Error for DecodeError {}

/// Detect and decode: EF BB BF ⇒ Utf8Bom; FF FE ⇒ Utf16Le; FE FF ⇒ Utf16Be; otherwise Utf8.
/// The BOM is not part of the text. Invalid input ⇒ Err with the byte offset (in `bytes`,
/// BOM included) of the first byte of the first invalid sequence or unpaired surrogate unit, or
/// of the trailing odd byte of a UTF-16 input.
///
/// Only one BOM is stripped: a second one is text (U+FEFF). Nothing is ever replaced.
pub fn decode(bytes: &[u8]) -> Result<(String, Encoding), DecodeError> {
    let encoding = detect(bytes);
    let start = encoding.bom().len();
    let body = &bytes[start..];
    let text = match encoding {
        Encoding::Utf8 | Encoding::Utf8Bom => match std::str::from_utf8(body) {
            Ok(s) => s.to_owned(),
            Err(e) => {
                return Err(DecodeError {
                    encoding,
                    byte_offset: start + e.valid_up_to(),
                });
            }
        },
        Encoding::Utf16Le => decode_utf16(body, start, encoding, u16::from_le_bytes)?,
        Encoding::Utf16Be => decode_utf16(body, start, encoding, u16::from_be_bytes)?,
    };
    Ok((text, encoding))
}

fn detect(bytes: &[u8]) -> Encoding {
    for encoding in [Encoding::Utf8Bom, Encoding::Utf16Le, Encoding::Utf16Be] {
        if bytes.starts_with(encoding.bom()) {
            return encoding;
        }
    }
    Encoding::Utf8
}

/// Strict UTF-16 decoding of `body`, which starts at `start` in the original input.
fn decode_utf16(
    body: &[u8],
    start: usize,
    encoding: Encoding,
    unit_of: fn([u8; 2]) -> u16,
) -> Result<String, DecodeError> {
    let err = |offset: usize| DecodeError {
        encoding,
        byte_offset: start + offset,
    };
    let units = body.chunks_exact(2).map(|c| unit_of([c[0], c[1]]));
    let mut text = String::with_capacity(body.len() / 2);
    // Index (in units) of the next unit to decode.
    let mut unit_index = 0;
    for decoded in char::decode_utf16(units) {
        match decoded {
            Ok(c) => {
                text.push(c);
                unit_index += c.len_utf16();
            }
            // The error is reported at the unpaired surrogate itself, which is the next unit.
            Err(_) => return Err(err(2 * unit_index)),
        }
    }
    if body.len() % 2 == 1 {
        return Err(err(body.len() - 1));
    }
    Ok(text)
}

/// Inverse of decode: BOM (if the encoding has one) followed by the text in that encoding.
pub fn encode(text: &str, encoding: Encoding) -> Vec<u8> {
    let mut out = encoding.bom().to_vec();
    match encoding {
        Encoding::Utf8 | Encoding::Utf8Bom => out.extend_from_slice(text.as_bytes()),
        Encoding::Utf16Le => {
            out.reserve(text.len() * 2);
            for unit in text.encode_utf16() {
                out.extend_from_slice(&unit.to_le_bytes());
            }
        }
        Encoding::Utf16Be => {
            out.reserve(text.len() * 2);
            for unit in text.encode_utf16() {
                out.extend_from_slice(&unit.to_be_bytes());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Encoding; 4] = [
        Encoding::Utf8,
        Encoding::Utf8Bom,
        Encoding::Utf16Le,
        Encoding::Utf16Be,
    ];

    fn de(encoding: Encoding, byte_offset: usize) -> DecodeError {
        DecodeError {
            encoding,
            byte_offset,
        }
    }

    #[test]
    fn names() {
        let names: Vec<_> = ALL.iter().map(|e| e.name()).collect();
        assert_eq!(names, ["utf-8", "utf-8-bom", "utf-16le", "utf-16be"]);
    }

    #[test]
    fn detects_boms_and_strips_exactly_one() {
        assert_eq!(decode(b"").unwrap(), (String::new(), Encoding::Utf8));
        assert_eq!(decode(b"abc").unwrap(), ("abc".into(), Encoding::Utf8));
        assert_eq!(
            decode(&[0xEF, 0xBB, 0xBF]).unwrap(),
            (String::new(), Encoding::Utf8Bom)
        );
        assert_eq!(
            decode(&[0xEF, 0xBB, 0xBF, 0xEF, 0xBB, 0xBF, b'x']).unwrap(),
            ("\u{feff}x".into(), Encoding::Utf8Bom)
        );
        assert_eq!(
            decode(&[0xFF, 0xFE]).unwrap(),
            (String::new(), Encoding::Utf16Le)
        );
        assert_eq!(
            decode(&[0xFE, 0xFF]).unwrap(),
            (String::new(), Encoding::Utf16Be)
        );
        // A partial BOM is just (invalid) UTF-8.
        assert_eq!(decode(&[0xEF, 0xBB]), Err(de(Encoding::Utf8, 0)));
    }

    #[test]
    fn utf16_with_surrogate_pair() {
        let le = [0xFF, 0xFE, b'a', 0, 0x3D, 0xD8, 0x00, 0xDE, b'\n', 0];
        assert_eq!(decode(&le).unwrap(), ("a😀\n".into(), Encoding::Utf16Le));
        let be = [0xFE, 0xFF, 0, b'a', 0xD8, 0x3D, 0xDE, 0x00, 0, b'\r'];
        assert_eq!(decode(&be).unwrap(), ("a😀\r".into(), Encoding::Utf16Be));
        assert_eq!(encode("a😀\n", Encoding::Utf16Le), le);
        assert_eq!(encode("a😀\r", Encoding::Utf16Be), be);
    }

    #[test]
    fn exact_error_offsets() {
        assert_eq!(decode(b"ab\xffcd"), Err(de(Encoding::Utf8, 2)));
        assert_eq!(
            decode(&[0xEF, 0xBB, 0xBF, 0x61, 0xC3]),
            Err(de(Encoding::Utf8Bom, 4))
        );
        assert_eq!(decode(b"a\xe2\x82"), Err(de(Encoding::Utf8, 1)));
        assert_eq!(
            decode(&[0xFF, 0xFE, 0x61, 0x00, 0x62]),
            Err(de(Encoding::Utf16Le, 4))
        );
        assert_eq!(
            decode(&[0xFF, 0xFE, 0x00, 0xD8, 0x61, 0x00]),
            Err(de(Encoding::Utf16Le, 2))
        );
        assert_eq!(
            decode(&[0xFE, 0xFF, 0xDC, 0x00]),
            Err(de(Encoding::Utf16Be, 2))
        );
        // High surrogate as the last complete unit.
        assert_eq!(
            decode(&[0xFE, 0xFF, 0, b'a', 0xD8, 0x00]),
            Err(de(Encoding::Utf16Be, 4))
        );
        // An unpaired surrogate before a trailing odd byte is reported first.
        assert_eq!(
            decode(&[0xFF, 0xFE, 0x00, 0xDC, 0x61]),
            Err(de(Encoding::Utf16Le, 2))
        );
        // Overlong encoding and an encoded surrogate are invalid UTF-8.
        assert_eq!(decode(b"x\xc0\x80"), Err(de(Encoding::Utf8, 1)));
        assert_eq!(decode(b"xy\xed\xa0\x80"), Err(de(Encoding::Utf8, 2)));
    }

    #[test]
    fn decode_error_display() {
        assert_eq!(
            de(Encoding::Utf16Be, 7).to_string(),
            "invalid utf-16be at byte offset 7"
        );
    }

    #[test]
    fn encode_decode_inverse_on_generated_strings() {
        let alphabet = [
            'a', '\n', '\r', '\0', 'é', '€', '😀', '\u{feff}', '\u{ffff}', 'z',
        ];
        let mut seed: u64 = 0x7002;
        let mut next = move |bound: usize| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 33) as usize) % bound.max(1)
        };
        for _ in 0..500 {
            let len = next(12);
            let s: String = (0..len).map(|_| alphabet[next(alphabet.len())]).collect();
            for e in ALL {
                let got = decode(&encode(&s, e)).unwrap();
                if e == Encoding::Utf8 && s.starts_with('\u{feff}') {
                    assert_eq!(
                        got,
                        (s['\u{feff}'.len_utf8()..].to_owned(), Encoding::Utf8Bom)
                    );
                } else {
                    assert_eq!(got, (s.clone(), e));
                }
            }
        }
    }
}
