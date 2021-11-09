use indexmap::{set::IndexSet, IndexMap};
use std::collections::BTreeMap;

mod dwarf;
mod error;
mod lookup;
mod serialize;

#[derive(Debug, Default)]
pub struct Converter {
    string_bytes: Vec<u8>,
    strings: IndexMap<Vec<u8>, String>,
    files: IndexSet<File>,
    functions: IndexSet<Function>,
    source_locations: IndexSet<SourceLocation>,
    // TODO: save "unfinished" source locations directly here, and concat them in the serializer
    ranges: BTreeMap<u32, u32>,
}

impl Converter {
    //     pub fn transform_strings<F: FnMut(String) -> String>(&mut self, _mapper: F) {
    //         // TODO: transform all the strings, for example to apply BCSymbolMaps.
    //     }
    fn insert_string(&mut self, s: &[u8]) -> u32 {
        if let Some(existing_idx) = self.strings.get_index_of(s) {
            return existing_idx as u32;
        }
        let string_offset = self.string_bytes.len() as u32;
        let string_len = s.len() as u32;
        self.string_bytes.extend(s);
        let (string_idx, _) = self.strings.insert_full(
            s.to_owned(),
            String {
                string_offset,
                string_len,
            },
        );
        string_idx as u32
    }
}

// TODO: maybe later, move all the casting to `u32` from the processor to the serializer

#[derive(Debug)]
struct String {
    string_offset: u32,
    string_len: u32,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct File {
    directory_idx: Option<u32>,
    path_name_idx: u32,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Function {
    name_idx: u32,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct SourceLocation {
    file_idx: u32,
    line: u32,
    function_idx: u32,
    inlined_into_idx: Option<u32>,
}

impl Converter {
    pub fn new() -> Self {
        Self::default()
    }
}
