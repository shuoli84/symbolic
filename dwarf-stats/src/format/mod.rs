use std::{mem, ptr};

mod error;
mod lookup;
pub(crate) mod raw;

pub use error::Error;
use raw::align_to_eight;

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Format<'data> {
    strings: &'data [raw::String],
    files: &'data [raw::File],
    functions: &'data [raw::Function],
    source_locations: &'data [raw::SourceLocation],
    ranges: &'data [raw::Range],
    string_bytes: &'data [u8],
}

impl<'data> Format<'data> {
    /// Parse the symcache binary format into a convenient type that allows safe access and allows
    /// fast lookups.
    ///
    /// See the [raw module](raw) for an explanation of the binary format.
    pub fn parse(buf: &'data [u8]) -> Result<Self> {
        if align_to_eight(buf.as_ptr() as usize) != 0 {
            return Err(Error::BufferNotAligned);
        }

        let mut header_size = mem::size_of::<raw::Header>();
        header_size += align_to_eight(header_size);

        if buf.len() < header_size {
            return Err(Error::HeaderTooSmall);
        }
        // SAFETY: we checked that the buffer is well aligned and large enough to fit a `raw::Header`.
        let header = unsafe { &*(buf.as_ptr() as *const raw::Header) };
        // TODO: check preamble, endianness and version
        // if header.version != FORMAT_VERSION {
        //     return Err(Error::WrongVersion);
        // }

        let mut strings_size = mem::size_of::<raw::String>() * header.num_strings as usize;
        strings_size += align_to_eight(strings_size);

        let mut files_size = mem::size_of::<raw::File>() * header.num_files as usize;
        files_size += align_to_eight(files_size);

        let mut functions_size = mem::size_of::<raw::Function>() * header.num_functions as usize;
        functions_size += align_to_eight(functions_size);

        let mut source_locations_size =
            mem::size_of::<raw::SourceLocation>() * header.num_source_locations as usize;
        source_locations_size += align_to_eight(source_locations_size);

        let mut ranges_size = mem::size_of::<raw::Range>() * header.num_ranges as usize;
        ranges_size += align_to_eight(ranges_size);

        let expected_buf_size = header_size
            + strings_size
            + files_size
            + functions_size
            + source_locations_size
            + ranges_size
            + header.string_bytes as usize;

        if buf.len() != expected_buf_size || source_locations_size < ranges_size {
            return Err(Error::BadFormatLength);
        }

        // SAFETY: we just made sure that all the pointers we are constructing via pointer
        // arithmetic are within `buf`
        let strings_start = unsafe { buf.as_ptr().add(header_size) };
        let files_start = unsafe { strings_start.add(strings_size) };
        let functions_start = unsafe { files_start.add(files_size) };
        let source_locations_start = unsafe { functions_start.add(functions_size) };
        let ranges_start = unsafe { source_locations_start.add(source_locations_size) };
        let string_bytes_start = unsafe { ranges_start.add(ranges_size) };

        // SAFETY: the above buffer size check also made sure we are not going out of bounds
        // here
        let strings = unsafe {
            &*(ptr::slice_from_raw_parts(strings_start, header.num_strings as usize)
                as *const [raw::String])
        };
        let files = unsafe {
            &*(ptr::slice_from_raw_parts(files_start, header.num_files as usize)
                as *const [raw::File])
        };
        let functions = unsafe {
            &*(ptr::slice_from_raw_parts(functions_start, header.num_functions as usize)
                as *const [raw::Function])
        };
        let source_locations = unsafe {
            &*(ptr::slice_from_raw_parts(
                source_locations_start,
                header.num_source_locations as usize,
            ) as *const [raw::SourceLocation])
        };
        let ranges = unsafe {
            &*(ptr::slice_from_raw_parts(ranges_start, header.num_ranges as usize)
                as *const [raw::Range])
        };
        let string_bytes = unsafe {
            &*(ptr::slice_from_raw_parts(string_bytes_start, header.string_bytes as usize)
                as *const [u8])
        };

        Ok(Format {
            strings,
            files,
            functions,
            source_locations,
            ranges,
            string_bytes,
        })
    }

    fn get_string(&self, string_idx: u32) -> Result<Option<&str>> {
        if string_idx == u32::MAX {
            return Ok(None);
        }
        let string = self
            .strings
            .get(string_idx as usize)
            .ok_or(Error::InvalidStringReference(string_idx))?;

        let start_offset = string.string_offset as usize;
        let end_offset = start_offset + string.string_len as usize;
        let bytes = self
            .string_bytes
            .get(start_offset..end_offset)
            .ok_or(Error::InvalidStringDataReference(string_idx))?;

        let s =
            std::str::from_utf8(bytes).map_err(|err| Error::InvalidStringData(string_idx, err))?;

        Ok(Some(s))
    }
}
