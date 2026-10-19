use wow_adt::adt::FlightBounds;

pub(super) fn header(magic: &[u8; 4], count: usize) -> Vec<u8> {
    let mut bytes = magic.to_vec();
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&u32::try_from(count).unwrap().to_le_bytes());
    bytes
}

pub(super) fn append_float(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend_from_slice(&value.to_bits().to_le_bytes());
}

pub(super) fn server_record(fdid: u32, min: Option<[i16; 9]>) -> Vec<u8> {
    let mut bytes = fdid.to_le_bytes().to_vec();
    bytes.push(u8::from(min.is_some()));
    for value in min.unwrap_or([0; 9]) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    assert_eq!(bytes.len(), 23);
    bytes
}

pub(super) fn client_record(fdid: u32, bounds: Option<FlightBounds>) -> Vec<u8> {
    let mut bytes = fdid.to_le_bytes().to_vec();
    bytes.push(u8::from(bounds.is_some()));
    let bounds = bounds.unwrap_or(FlightBounds {
        min_heights: [0; 9],
        max_heights: [0; 9],
    });
    for value in bounds.max_heights.into_iter().chain(bounds.min_heights) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    assert_eq!(bytes.len(), 41);
    bytes
}

pub(super) fn outcome(magic: &[u8; 4], result: &Result<Vec<u8>, String>) -> Vec<u8> {
    match result {
        Ok(record) => {
            let mut bytes = header(magic, 1);
            bytes.extend_from_slice(record);
            bytes
        }
        Err(error) => {
            let mut bytes = header(b"MFBE", 1);
            bytes.extend_from_slice(&(error.len() as u32).to_le_bytes());
            bytes.extend_from_slice(error.as_bytes());
            bytes
        }
    }
}

pub(super) fn selected_payload(payload: Option<&[u8]>) -> Vec<u8> {
    let mut bytes = header(b"MFBR", 1);
    bytes.push(u8::from(payload.is_some()));
    let payload = payload.unwrap_or(&[]);
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}
