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
    #[error("Expected a floating-point number at position {i}")]
    F64 { i: u64 },
    #[error("Failed to read {:?} as valid floating-point number", String::from_utf8_lossy(&raw))]
    F64Value { raw: Vec<u8>, err: std::num::ParseFloatError },
    #[error("Failed to read from reader")]
    Read(#[source] std::io::Error),
    #[error("Expected a keyword at position {i}")]
    Keyword { i: u64 },
    #[error("Expected an unsigned number at position {i}")]
    U32 { i: u64 },
    #[error("Failed to read {:?} as valid unsigned number", String::from_utf8_lossy(&raw))]
    U32Value { raw: Vec<u8>, err: std::num::ParseIntError },
}





/***** LIBRARY *****/
/// A wrapper around a [`Read`]er that tokenizes the input stream useful for parsing Wavefront's
/// `.obj` and `.mtl` file formats.
///
/// # Generics
/// - `R`: The type of the wrapper reader.
pub struct Tokenizer<R> {
    reader: R,
    pub i:  u64,
}
impl<R> Tokenizer<R> {
    /// Constructor for the Tokenizer.
    ///
    /// # Arguments
    /// - `reader`: The `R`eader to wrap the Tokenizer around.
    ///
    /// # Returns
    /// A new Tokenizer that can parse tokens off the stream.
    pub const fn new(reader: R) -> Self { Self { reader, i: 1 } }
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
        let mut b: u8 = 0;
        if self.reader.read(std::slice::from_mut(&mut b)).map_err(Error::Read)? > 0 {
            let i: u64 = self.i;
            self.i += 1;
            Ok(Some((i, b)))
        } else {
            Ok(None)
        }
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
            return Err(Error::Keyword { i });
        }

        // Then only read keyword bytes
        let mut keyword = vec![b];
        while let Some((_, b)) = self.next()? {
            if (b < b'a' || b > b'z') && (b < b'A' || b > b'Z') && b != b'_' {
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
            return Err(Error::U32 { i });
        }

        // Read until it's a newline
        let mut raw = vec![b];
        while let Some((_, b)) = self.next()? {
            if b < b'0' || b > b'9' {
                return Ok(Some(u32::from_str(String::from_utf8_lossy(&raw).trim()).map_err(|err| Error::U32Value { raw, err })?));
            }
            raw.push(b);
        }
        Ok(Some(u32::from_str(String::from_utf8_lossy(&raw).trim()).map_err(|err| Error::U32Value { raw, err })?))
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
                return Ok(Some(f64::from_str(String::from_utf8_lossy(&raw).trim()).map_err(|err| Error::F64Value { raw, err })?));
            }
            raw.push(b);
        }
        Ok(Some(f64::from_str(String::from_utf8_lossy(&raw).trim()).map_err(|err| Error::F64Value { raw, err })?))
    }
}
