//! Chunked client files and bounds-checked little-endian reads.

pub type Result<T> = std::result::Result<T, String>;

/// `(tag as stored, payload)` of each top-level chunk. ADT, WDT and WMO tags are stored
/// reversed (`MCNK` as `KNCM`); M2 tags are not.
pub fn chunks(data: &[u8]) -> impl Iterator<Item = ([u8; 4], &[u8])> {
    let mut offset = 0;
    std::iter::from_fn(move || {
        let tag: [u8; 4] = data.get(offset..offset + 4)?.try_into().ok()?;
        let size = u32::from_le_bytes(data.get(offset + 4..offset + 8)?.try_into().ok()?) as usize;
        let payload = data.get(offset + 8..offset + 8 + size)?;
        offset += 8 + size;
        Some((tag, payload))
    })
}

/// First chunk of an ADT/WDT/WMO file tagged `tag` (as documented, e.g. `MODF`).
pub fn find_reversed<'a>(data: &'a [u8], tag: &[u8; 4]) -> Option<&'a [u8]> {
    let stored = [tag[3], tag[2], tag[1], tag[0]];
    chunks(data)
        .find(|(found, _)| *found == stored)
        .map(|(_, payload)| payload)
}

/// First chunk of an M2-family file tagged `tag`.
pub fn find<'a>(data: &'a [u8], tag: &[u8; 4]) -> Option<&'a [u8]> {
    chunks(data)
        .find(|(found, _)| found == tag)
        .map(|(_, payload)| payload)
}

pub fn bytes<const N: usize>(data: &[u8], offset: usize) -> Result<[u8; N]> {
    data.get(offset..offset + N)
        .and_then(|slice| slice.try_into().ok())
        .ok_or_else(|| format!("read of {N} bytes at {offset:#x} past {} bytes", data.len()))
}

pub fn u16_at(data: &[u8], offset: usize) -> Result<u16> {
    bytes(data, offset).map(u16::from_le_bytes)
}

pub fn i16_at(data: &[u8], offset: usize) -> Result<i16> {
    bytes(data, offset).map(i16::from_le_bytes)
}

pub fn u32_at(data: &[u8], offset: usize) -> Result<u32> {
    bytes(data, offset).map(u32::from_le_bytes)
}

pub fn f32_at(data: &[u8], offset: usize) -> Result<f32> {
    bytes(data, offset).map(f32::from_le_bytes)
}

pub fn f32x3_at(data: &[u8], offset: usize) -> Result<[f32; 3]> {
    Ok([
        f32_at(data, offset)?,
        f32_at(data, offset + 4)?,
        f32_at(data, offset + 8)?,
    ])
}

/// `(count, offset)` of the M2Array header at `offset`.
pub fn array_at(data: &[u8], offset: usize) -> Result<(usize, usize)> {
    Ok((
        u32_at(data, offset)? as usize,
        u32_at(data, offset + 4)? as usize,
    ))
}

/// `count` records of `size` bytes from `offset`, each read by `read`.
pub fn records<T>(
    data: &[u8],
    (count, offset): (usize, usize),
    size: usize,
    read: impl Fn(&[u8], usize) -> Result<T>,
) -> Result<Vec<T>> {
    let end = count
        .checked_mul(size)
        .and_then(|len| len.checked_add(offset))
        .ok_or("record array size overflows")?;
    if end > data.len() {
        return Err(format!(
            "{count} records of {size} bytes at {offset:#x} run past {} bytes",
            data.len()
        ));
    }
    (0..count)
        .map(|index| read(data, offset + index * size))
        .collect()
}

pub fn u16s(data: &[u8]) -> Vec<u16> {
    data.as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect()
}

pub fn u32s(data: &[u8]) -> Vec<u32> {
    data.as_chunks::<4>()
        .0
        .iter()
        .map(|word| u32::from_le_bytes(*word))
        .collect()
}
