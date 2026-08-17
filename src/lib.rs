//  LIB.rs
//    by Lut99
//
//  Description:
//!   Common tokenizer module for <https://github.com/Lut99/obj-rs> and <https://github.com/Lut99/mtllib-rs>.
//

use std::io::Read;
use std::str::FromStr as _;

use thiserror::Error;


/***** ERRORS *****/
/// Defines errors occurring in the [`Tokenizer`].
#[derive(Debug, Error)]
pub enum Error {
    #[error("Expected a boolean value ('0', '1', 'on', 'off', 'true', 'false', 'yes' or 'no') at position {i}")]
    Bool { i: u64 },
    #[error("Expected 'on' or 'off' at position {i}")]
    BoolO { i: u64 },
    #[error("Expected byte {b:?} at position {i}")]
    Expect { i: u64, b: u8 },
    #[error("Expected whitespace at position {i}")]
    ExpectWhitespace { i: u64 },
    #[error("Expected a floating-point number at position {i}")]
    F64 { i: u64 },
    #[error("Failed to read {:?} as valid floating-point number", String::from_utf8_lossy(&raw))]
    F64Value { raw: Vec<u8>, err: std::num::ParseFloatError },
    #[error("Expected a signed number at position {i}")]
    ISize { i: u64 },
    #[error("Failed to read {:?} as valid signed number", String::from_utf8_lossy(&raw))]
    ISizeValue { raw: Vec<u8>, err: std::num::ParseIntError },
    #[error("Failed to read from reader")]
    Read(#[source] std::io::Error),
    #[error("Expected a keyword at position {i}")]
    Keyword { i: u64 },
    #[error("Expected an unsigned number at position {i}")]
    U32 { i: u64 },
    #[error("Failed to read {:?} as valid unsigned number", String::from_utf8_lossy(&raw))]
    U32Value { raw: Vec<u8>, err: std::num::ParseIntError },
    #[error("Expected an unsigned number at position {i}")]
    U64 { i: u64 },
    #[error("Failed to read {:?} as valid unsigned number", String::from_utf8_lossy(&raw))]
    U64Value { raw: Vec<u8>, err: std::num::ParseIntError },
}





/***** LIBRARY *****/
/// A wrapper around a [`Read`]er that tokenizes the input stream useful for parsing Wavefront's
/// `.obj` and `.mtl` file formats.
///
/// # Generics
/// - `R`: The type of the wrapper reader.
pub struct Tokenizer<R> {
    reader:  R,
    putback: Vec<u8>,
    pub i:   u64,
}
impl<R> Tokenizer<R> {
    /// Constructor for the Tokenizer.
    ///
    /// # Arguments
    /// - `reader`: The `R`eader to wrap the Tokenizer around.
    ///
    /// # Returns
    /// A new Tokenizer that can parse tokens off the stream.
    pub const fn new(reader: R) -> Self { Self { reader, putback: Vec::new(), i: 1 } }
}
impl<R: Read> Tokenizer<R> {
    /// Gets anything off the stream.
    ///
    /// This is just a wrapper around getting a single byte off of `R`, but then such that internal
    /// position counters are updated accordingly.
    ///
    /// # Returns
    /// The index of the next byte and the byte, or [`None`].
    ///
    /// # Errors
    /// This function fails if the underlying `R`eader fails.
    pub fn next(&mut self) -> Result<Option<(u64, u8)>, Error> {
        // First check if we need to return any putbacks
        if let Some(b) = self.putback.pop() {
            let i: u64 = self.i;
            self.i += 1;
            return Ok(Some((i, b)));
        }

        // Then pop off the stream
        let mut b: u8 = 0;
        if self.reader.read(std::slice::from_mut(&mut b)).map_err(Error::Read)? > 0 {
            let i: u64 = self.i;
            self.i += 1;
            Ok(Some((i, b)))
        } else {
            Ok(None)
        }
    }

    /// Gets the next mandatory character sequence off the stream.
    ///
    /// This does NOT ignore comments & whitespace!
    ///
    /// # Arguments
    /// - `bs`: The bytes to expect.
    ///
    /// # Errors
    /// This function fails if the underlying `R`eader fails, or else if the stream was not headed
    /// by the given `bs`.
    pub fn expect(&mut self, bs: &[u8]) -> Result<(), Error> {
        let mut i: usize = 0;
        while i < bs.len() {
            let next: u8 = self
                .next()?
                .ok_or_else(|| {
                    while i > 0 {
                        self.putback.push(bs[i]);
                        self.i -= 1;
                    }
                    Error::Expect { i: self.i, b: bs[i] }
                })?
                .1;
            if next != bs[i] {
                self.putback.push(next);
                self.i -= 1;
                while i > 0 {
                    self.putback.push(bs[i]);
                    self.i -= 1;
                }
                return Err(Error::Expect { i: self.i, b: bs[i] });
            }
            i += 1;
        }
        Ok(())
    }

    /// Pops a single whitespace character off the stream.
    ///
    /// This can be used to express that a punctuation is expected.
    ///
    /// # Errors
    /// This function fails if the underlying `R`eader fails, or else if the stream was not headed
    /// by a whitespace
    pub fn expect_whitespace(&mut self) -> Result<(), Error> {
        enum State {
            Start,
            Comment,
        }

        let mut state = State::Start;
        while let Some((i, b)) = self.next()? {
            match state {
                State::Start if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' => return Ok(()),
                State::Start if b == b'#' => state = State::Comment,
                State::Start => {
                    self.putback.push(b);
                    self.i -= 1;
                    return Err(Error::ExpectWhitespace { i });
                },

                State::Comment if b == b'\n' => return Ok(()),
                State::Comment => continue,
            }
        }
        Ok(())
    }

    /// Gets the next character off the stream.
    ///
    /// This ignores comments & whitespace up to the first byte that is not one of those.
    ///
    /// # Returns
    /// The byte or [`None`].
    ///
    /// # Errors
    /// This function fails if the underlying `R`eader fails.
    pub fn byte(&mut self) -> Result<Option<(u64, u8)>, Error> {
        enum State {
            Start,
            Comment,
        }

        let mut state = State::Start;
        while let Some((i, b)) = self.next()? {
            match state {
                State::Start if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' => continue,
                State::Start if b == b'#' => state = State::Comment,
                State::Start => return Ok(Some((i, b))),

                State::Comment if b == b'\n' => state = State::Start,
                State::Comment => continue,
            }
        }
        Ok(None)
    }

    /// Gets a keyword off the stream.
    ///
    /// This ignores comments & whitespace up to the first keyword char.
    ///
    /// # Returns
    /// The keyword as a raw sequence of bytes or [`None`].
    ///
    /// # Errors
    /// This function fails if the underlying `R`eader fails or if no alphabetical character was at
    /// the top of the stream.
    pub fn keyword(&mut self) -> Result<Option<Vec<u8>>, Error> {
        // Get a first byte that's a keyword byte.
        let (i, b): (u64, u8) = match self.byte()? {
            Some(b) => b,
            None => return Ok(None),
        };
        if (b < b'a' || b > b'z') && (b < b'A' || b > b'Z') {
            self.putback.push(b);
            self.i -= 1;
            return Err(Error::Keyword { i });
        }

        // Then only read keyword bytes
        let mut keyword = vec![b];
        while let Some((_, b)) = self.next()? {
            if (b < b'a' || b > b'z') && (b < b'A' || b > b'Z') && b != b'_' {
                self.putback.push(b);
                self.i -= 1;
                return Ok(Some(keyword));
            }
            keyword.push(b);
        }
        Ok(Some(keyword))
    }

    /// Pop a single string off the stream.
    ///
    /// That is, the remainder of the line after a whitespace.
    ///
    /// This ignores comments & whitespace up to the first non-whitespace char.
    ///
    /// # Returns
    /// The string or [`None`].
    ///
    /// # Errors
    /// This function fails if the underlying `R`eader fails.
    pub fn string(&mut self) -> Result<Option<String>, Error> {
        // Get a first byte
        let b: u8 = match self.byte()? {
            Some((_, b)) => b,
            None => return Ok(None),
        };

        // Read until it's a newline
        let mut s = vec![b];
        while let Some((_, b)) = self.next()? {
            if b == b'\n' {
                return Ok(Some(String::from_utf8_lossy(&s).trim().into()));
            }
            s.push(b);
        }
        Ok(Some(String::from_utf8_lossy(&s).trim().into()))
    }

    /// Pop a single boolean off the stream.
    ///
    /// True values are `1`, `on`, `true` and `yes`. False values are `0`, `off`, `false` and `no`.
    ///
    /// This ignores comments & whitespace up to the first valid char.
    ///
    /// # Returns
    /// The bool or [`None`].
    ///
    /// # Errors
    /// This function fails if the underlying `R`eader fails or if the stream was not spearheaded
    /// by a valid bool char
    pub fn bool(&mut self) -> Result<Option<bool>, Error> {
        // Get a first byte in the range
        let (i, b): (u64, u8) = match self.byte()? {
            Some(b) => b,
            None => return Ok(None),
        };
        match b {
            // Digits
            b'1' => self.expect_whitespace().map(|_| Some(true)),
            b'0' => self.expect_whitespace().map(|_| Some(false)),

            // Power states
            b'o' => match self.next()? {
                Some((_, b'n')) => self.expect_whitespace().map(|_| Some(true)),
                Some((_, b'f')) => {
                    self.expect(b"f")?;
                    self.expect_whitespace().map(|_| Some(false))
                },
                _ => Err(Error::BoolO { i }),
            },

            // Booleans
            b't' => {
                self.expect(b"rue")?;
                self.expect_whitespace().map(|_| Some(true))
            },
            b'f' => {
                self.expect(b"alse")?;
                self.expect_whitespace().map(|_| Some(false))
            },

            // Answers
            b'y' => {
                self.expect(b"es")?;
                self.expect_whitespace().map(|_| Some(true))
            },
            b'n' => {
                self.expect(b"o")?;
                self.expect_whitespace().map(|_| Some(false))
            },

            // Anything else is unexpected
            _ => {
                self.putback.push(b);
                self.i -= 1;
                Err(Error::Bool { i })
            },
        }
    }

    /// Pop a single (unsigned) integer number off the stream.
    ///
    /// This ignores comments & whitespace up to the first digit.
    ///
    /// # Returns
    /// The unsinged integer or [`None`].
    ///
    /// # Errors
    /// This function fails if the underlying `R`eader fails or if the stream was not spearheaded
    /// by a digit.
    pub fn u32(&mut self) -> Result<Option<u32>, Error> {
        // Get a first byte in the range
        let (i, b): (u64, u8) = match self.byte()? {
            Some(b) => b,
            None => return Ok(None),
        };
        if b < b'0' || b > b'9' {
            self.putback.push(b);
            self.i -= 1;
            return Err(Error::U32 { i });
        }

        // Read until it's a newline
        let mut raw = vec![b];
        while let Some((_, b)) = self.next()? {
            if b < b'0' || b > b'9' {
                self.putback.push(b);
                self.i -= 1;
                return Ok(Some(u32::from_str(String::from_utf8_lossy(&raw).trim()).map_err(|err| {
                    for b in raw.iter().rev() {
                        self.putback.push(*b);
                        self.i -= 1;
                    }
                    Error::U32Value { raw, err }
                })?));
            }
            raw.push(b);
        }
        Ok(Some(u32::from_str(String::from_utf8_lossy(&raw).trim()).map_err(|err| {
            for b in raw.iter().rev() {
                self.putback.push(*b);
                self.i -= 1;
            }
            Error::U32Value { raw, err }
        })?))
    }

    /// Pop a single (unsigned) long number off the stream.
    ///
    /// This ignores comments & whitespace up to the first digit.
    ///
    /// # Returns
    /// The unsinged long or [`None`].
    ///
    /// # Errors
    /// This function fails if the underlying `R`eader fails or if the stream was not spearheaded
    /// by a digit.
    pub fn u64(&mut self) -> Result<Option<u64>, Error> {
        // Get a first byte in the range
        let (i, b): (u64, u8) = match self.byte()? {
            Some(b) => b,
            None => return Ok(None),
        };
        if b < b'0' || b > b'9' {
            self.putback.push(b);
            self.i -= 1;
            return Err(Error::U64 { i });
        }

        // Read until it's a newline
        let mut raw = vec![b];
        while let Some((_, b)) = self.next()? {
            if b < b'0' || b > b'9' {
                self.putback.push(b);
                self.i -= 1;
                return Ok(Some(u64::from_str(String::from_utf8_lossy(&raw).trim()).map_err(|err| {
                    for b in raw.iter().rev() {
                        self.putback.push(*b);
                        self.i -= 1;
                    }
                    Error::U64Value { raw, err }
                })?));
            }
            raw.push(b);
        }
        Ok(Some(u64::from_str(String::from_utf8_lossy(&raw).trim()).map_err(|err| {
            for b in raw.iter().rev() {
                self.putback.push(*b);
                self.i -= 1;
            }
            Error::U64Value { raw, err }
        })?))
    }

    /// Pop a single (signed) address-length number off the stream.
    ///
    /// This ignores comments & whitespace up to the first digit.
    ///
    /// # Returns
    /// The unsinged long or [`None`].
    ///
    /// # Errors
    /// This function fails if the underlying `R`eader fails or if the stream was not spearheaded
    /// by a digit.
    pub fn isize(&mut self) -> Result<Option<isize>, Error> {
        // Get a first byte in the range
        let (i, b): (u64, u8) = match self.byte()? {
            Some(b) => b,
            None => return Ok(None),
        };
        if b < b'0' || b > b'9' {
            self.putback.push(b);
            self.i -= 1;
            return Err(Error::ISize { i });
        }

        // Read until it's a newline
        let mut raw = vec![b];
        while let Some((_, b)) = self.next()? {
            if b < b'0' || b > b'9' {
                self.putback.push(b);
                self.i -= 1;
                return Ok(Some(isize::from_str(String::from_utf8_lossy(&raw).trim()).map_err(|err| {
                    for b in raw.iter().rev() {
                        self.putback.push(*b);
                        self.i -= 1;
                    }
                    Error::ISizeValue { raw, err }
                })?));
            }
            raw.push(b);
        }
        Ok(Some(isize::from_str(String::from_utf8_lossy(&raw).trim()).map_err(|err| {
            for b in raw.iter().rev() {
                self.putback.push(*b);
                self.i -= 1;
            }
            Error::ISizeValue { raw, err }
        })?))
    }

    /// Pop a single floating-point number off the stream.
    ///
    /// This ignores comments & whitespace up to the first digit.
    ///
    /// # Returns
    /// The unsinged float or [`None`].
    ///
    /// # Errors
    /// This function fails if the underlying `R`eader fails or if the stream was not spearheaded
    /// by a digit or period (`.`).
    pub fn f64(&mut self) -> Result<Option<f64>, Error> {
        // Get a first byte in the range
        let (i, b): (u64, u8) = match self.byte()? {
            Some(b) => b,
            None => return Ok(None),
        };
        if (b < b'0' || b > b'9') && b != b'.' {
            return Err(Error::F64 { i });
        }

        // Read until it's a newline
        let mut raw = vec![b];
        while let Some((_, b)) = self.next()? {
            if (b < b'0' || b > b'9') && b != b'.' {
                self.putback.push(b);
                self.i -= 1;
                return Ok(Some(f64::from_str(String::from_utf8_lossy(&raw).trim()).map_err(|err| {
                    for b in raw.iter().rev() {
                        self.putback.push(*b);
                        self.i -= 1;
                    }
                    Error::F64Value { raw, err }
                })?));
            }
            raw.push(b);
        }
        Ok(Some(f64::from_str(String::from_utf8_lossy(&raw).trim()).map_err(|err| {
            for b in raw.iter().rev() {
                self.putback.push(*b);
                self.i -= 1;
            }
            Error::F64Value { raw, err }
        })?))
    }
}





/***** TESTS *****/
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool() {
        let eg1 = b"1";
        let eg2 = b"on";
        let eg3 = b"true";
        let eg4 = b"yes";
        let eg5 = b"0";
        let eg6 = b"off";
        let eg7 = b"false";
        let eg8 = b"no";
        let eg9 = b"a";
        let eg10 = b"o";
        let eg11 = b"10";
        let eg12 = b"falser";

        assert!(matches!(Tokenizer::new(eg1.as_slice()).bool(), Ok(Some(true))));
        assert!(matches!(Tokenizer::new(eg2.as_slice()).bool(), Ok(Some(true))));
        assert!(matches!(Tokenizer::new(eg3.as_slice()).bool(), Ok(Some(true))));
        assert!(matches!(Tokenizer::new(eg4.as_slice()).bool(), Ok(Some(true))));
        assert!(matches!(Tokenizer::new(eg5.as_slice()).bool(), Ok(Some(false))));
        assert!(matches!(Tokenizer::new(eg6.as_slice()).bool(), Ok(Some(false))));
        assert!(matches!(Tokenizer::new(eg7.as_slice()).bool(), Ok(Some(false))));
        assert!(matches!(Tokenizer::new(eg8.as_slice()).bool(), Ok(Some(false))));
        assert!(matches!(Tokenizer::new(eg9.as_slice()).bool(), Err(Error::Bool { i: 1 })));
        assert!(matches!(Tokenizer::new(eg10.as_slice()).bool(), Err(Error::BoolO { i: 1 })));
        assert!(matches!(Tokenizer::new(eg11.as_slice()).bool(), Err(Error::ExpectWhitespace { i: 2 })));
        assert!(matches!(Tokenizer::new(eg12.as_slice()).bool(), Err(Error::ExpectWhitespace { i: 6 })));
    }
}
