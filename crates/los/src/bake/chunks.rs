//! Server chunk-selection policy over shared iteration and byte readers.
pub use wow_chunk::{read_f32 as f32_at, read_u32 as u32_at};
pub type Result<T> = std::result::Result<T, String>;

/// Legacy selection stops at a truncated chunk; typed format parsers report their errors.
pub fn chunks(data: &[u8]) -> impl Iterator<Item = ([u8; 4], &[u8])> {
    wow_chunk::ChunkIter::new(data)
        .map_while(std::result::Result::ok)
        .map(|(tag, payload)| (*tag, payload))
}

pub fn find_reversed<'a>(data: &'a [u8], tag: &[u8; 4]) -> Option<&'a [u8]> {
    let stored = [tag[3], tag[2], tag[1], tag[0]];
    chunks(data)
        .find(|(found, _)| *found == stored)
        .map(|(_, payload)| payload)
}

pub fn find<'a>(data: &'a [u8], tag: &[u8; 4]) -> Option<&'a [u8]> {
    chunks(data)
        .find(|(found, _)| found == tag)
        .map(|(_, payload)| payload)
}
