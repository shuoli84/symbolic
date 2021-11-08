use indexmap::set::IndexSet;
use std::collections::BTreeMap;

mod dwarf;
mod lookup;

#[derive(Debug, Default)]
pub struct Converter {
    strings: IndexSet<String>,
    files: IndexSet<File>,
    functions: IndexSet<Function>,
    ranges: BTreeMap<u32, u32>,
    source_locations: IndexSet<SourceLocation>,
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
