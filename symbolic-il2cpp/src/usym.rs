//! Parser for the UsymLite format.
//!
//! This format can map il2cpp instruction addresses to managed file names and line numbers.
//!
//! Current state: This can parse the UsymLite format, a method to get the source location
//! based on the native instruction poiter does not yet exist.

use std::borrow::Cow;
use std::mem;
use std::ptr;

use anyhow::{Error, Result};

/// The header of the usym file format.
#[derive(Debug, Clone)]
#[repr(C)]
struct UsymHeader {
    /// Magic number identifying the file, `b"usym"`.  Probably.
    magic: u32,

    /// Version of the usym file format.  Probably.
    version: u32,

    /// Number of [`UsymRecord`] entries.
    ///
    /// These follow right after the header, and after them is the string table.
    record_count: u32,

    /// UUID of the assembly(?).
    id: u32,

    /// Name of something, the "assembly"?  Offset into string table.
    name: u32,

    /// Name of OS.  Offset into string table.
    os: u32,

    /// Name of architecture.  Offset into string table.
    arch: u32,
}

/// A record mapping an IL2CPP instruction address to managed code location.
///
/// This is the raw record as it appears in the file, see [`UsymRecord`] for a record with
/// the names resolved.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct UsymRawSourceRecord {
    /// Instruction pointer address, relative to ...?
    address: u64,
    /// Managed symbol name as offset in string table.
    symbol: u32,
    /// Reference to the managed source file name in the string table.
    file: u32,
    /// Managed line number.
    line: u32,
    // These might not even be u64, it's just 128 bits we don't know.
    _unknown0: u64,
    _unknown1: u64,
}

/// A record mapping an IL2CPP instruction address to managed code location.
#[derive(Debug, Clone)]
struct UsymSourceRecord<'a> {
    /// Instruction pointer address, relative to ...?
    address: u64,
    /// Symbol name of the managed code.
    symbol: Cow<'a, str>,
    /// File name of the managed code.
    file: Cow<'a, str>,
    /// Line number of the managed code.
    line: u32,
}

/// Reader for the usym symbols file format.
pub struct UsymSymbols<'a> {
    /// File header.
    header: &'a UsymHeader,
    /// Instruction address to managed code mapping records.
    records: &'a [UsymRawSourceRecord],
    /// The string table.
    ///
    /// This is a large slice of bytes with length-prefixed strings.  The length is a
    /// little-Indian u16.
    string_table: &'a [u8],
}

impl<'a> UsymSymbols<'a> {
    const MAGIC: &'static [u8] = b"usym";

    pub fn parse(buf: &'a [u8]) -> Result<UsymSymbols<'a>> {
        if buf.as_ptr().align_offset(8) != 0 {
            // Alignment only really matters for performance really.
            return Err(Error::msg("Data buffer not aligned to 8 bytes"));
        }
        if buf.len() < mem::size_of::<UsymHeader>() {
            return Err(Error::msg("Data smaller than UsymHeader"));
        }
        if buf.get(..4) != Some(Self::MAGIC) {
            return Err(Error::msg("Wrong magic number"));
        }

        // SAFETY: We checked the buffer is large enough above.
        let header = unsafe { &*(buf.as_ptr() as *const UsymHeader) };
        if header.version != 2 {
            return Err(Error::msg("Unknown version"));
        }

        let record_count: usize = header.record_count.try_into()?;
        let string_table_offset =
            mem::size_of::<UsymHeader>() + record_count * mem::size_of::<UsymRawSourceRecord>();
        if buf.len() < string_table_offset {
            return Err(Error::msg("Data smaller than number of records"));
        }

        // SAFETY: We checked the buffer is at least the size_of::<UsymHeader>() above.
        let first_record_ptr = unsafe { buf.as_ptr().add(mem::size_of::<UsymHeader>()) };

        // SAFETY: We checked the buffer has enough space for all the line records above.
        let records = unsafe {
            let first_record_ptr: *const UsymRawSourceRecord = first_record_ptr.cast();
            let records_ptr = ptr::slice_from_raw_parts(first_record_ptr, record_count);
            records_ptr
                .as_ref()
                .ok_or_else(|| Error::msg("lines_offset was null pointer!"))
        }?;

        let string_table = buf
            .get(string_table_offset..)
            .ok_or_else(|| Error::msg("No string table found"))?;

        Ok(Self {
            header,
            records,
            string_table,
        })
    }

    /// Returns a string from the string table at given offset.
    ///
    /// Offsets are as provided by some [`UsymLiteHeader`] and [`UsymLiteLine`] fields.
    fn get_string(&self, offset: usize) -> Option<Cow<'a, str>> {
        let size_bytes = self.string_table.get(offset..offset + 2)?;
        let size: usize = u16::from_le_bytes([size_bytes[0], size_bytes[1]]).into();

        let start_offset = offset + 2;
        let end_offset = start_offset + size;

        let string_bytes = self.string_table.get(start_offset..end_offset)?;
        Some(String::from_utf8_lossy(string_bytes))
    }

    /// Returns a [`UsymRecord`] at the given index it was stored.
    ///
    /// Not that useful, you have no idea what index you want.
    fn get_record(&self, index: usize) -> Option<UsymSourceRecord> {
        let raw = self.records.get(index)?;
        Some(UsymSourceRecord {
            address: raw.address,
            symbol: self.get_string(raw.symbol.try_into().unwrap())?,
            file: self.get_string(raw.file.try_into().unwrap())?,
            line: raw.line,
        })
    }

    /// Lookup the managed code source location for an IL2CPP instruction pointer.
    fn lookup_source_record(&self, ip: u64) -> Option<UsymSourceRecord> {
        // TODO: need to subtract the image base to get relative address
        match self.records.binary_search_by_key(&ip, |r| r.address) {
            Ok(index) => self.get_record(index),
            Err(index) => self.get_record(index - 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use symbolic_common::ByteView;
    use symbolic_testutils::fixture;

    use super::*;

    fn empty_usymlite() -> Result<ByteView<'static>> {
        let file = File::open(fixture("il2cpp/empty.usymlite"))?;
        ByteView::map_file_ref(&file).map_err(Into::into)
    }

    #[test]
    fn test_test() {
        let file = File::open(
            "/Users/flub/code/sentry-unity-il2cpp-line-numbers/Builds/iOS/UnityFramework.usym",
        )
        .unwrap();
        let data = ByteView::map_file_ref(&file).unwrap();
        let usyms = UsymSymbols::parse(&data).unwrap();

        println!("id: {}", usyms.get_string(0x02).unwrap());
        println!("name: {}", usyms.get_string(0x24).unwrap());
        println!("os: {}", usyms.get_string(0x34).unwrap());
        println!("arch: {}", usyms.get_string(0x39).unwrap());

        println!("{:?}", usyms.get_record(0));

        let record = usyms.lookup_source_record(2883352);
        println!("record: {record:?}");

        panic!("boom");
    }

    #[test]
    fn test_sorted_addresses() {
        let file = File::open(
            "/Users/flub/code/sentry-unity-il2cpp-line-numbers/Builds/iOS/UnityFramework.usym",
        )
        .unwrap();
        let data = ByteView::map_file_ref(&file).unwrap();
        let usyms = UsymSymbols::parse(&data).unwrap();

        let mut last_address = usyms.records[0].address;
        for i in 1usize..usyms.header.record_count as usize {
            assert!(usyms.records[i].address > last_address);
            last_address = usyms.records[0].address;
        }
    }
}
