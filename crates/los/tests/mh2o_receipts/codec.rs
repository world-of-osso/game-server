use los::liquid::ChunkLiquid;
use wow_adt::adt_tex::AdtWaterData;

pub(super) fn header(magic: &[u8; 4], count: usize) -> Vec<u8> {
    let mut bytes = magic.to_vec();
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&u32::try_from(count).unwrap().to_le_bytes());
    bytes
}

pub(super) fn append_blob(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&u32::try_from(value.len()).unwrap().to_le_bytes());
    bytes.extend_from_slice(value);
}

pub(super) fn append_float(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend_from_slice(&value.to_bits().to_le_bytes());
}

fn append_floats(bytes: &mut Vec<u8>, values: &[f32]) {
    bytes.extend_from_slice(&u32::try_from(values.len()).unwrap().to_le_bytes());
    for &value in values {
        append_float(bytes, value);
    }
}

pub(super) fn server_layers(chunks: &[ChunkLiquid]) -> Vec<u8> {
    let mut bytes = header(b"WMSL", chunks.len());
    for (index, chunk) in chunks.iter().enumerate() {
        bytes.extend_from_slice(&(index as u32).to_le_bytes());
        bytes.extend_from_slice(&(chunk.layers.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&chunk.deep.to_le_bytes());
        for layer in &chunk.layers {
            bytes.extend_from_slice(&layer.liquid_type.to_le_bytes());
            append_float(&mut bytes, layer.min_height);
            bytes.extend_from_slice(&[layer.x_offset, layer.y_offset, layer.width, layer.height]);
            bytes.extend_from_slice(&layer.exists);
            append_floats(&mut bytes, &layer.heights);
        }
    }
    bytes
}

pub(super) fn client_layers(data: &AdtWaterData) -> Vec<u8> {
    let mut bytes = header(b"WMCL", data.chunks.len());
    for (index, chunk) in data.chunks.iter().enumerate() {
        bytes.extend_from_slice(&(index as u32).to_le_bytes());
        bytes.extend_from_slice(&(chunk.layers.len() as u32).to_le_bytes());
        bytes.push(u8::from(chunk.attributes.is_some()));
        let attributes = chunk.attributes.unwrap_or_default();
        bytes.extend_from_slice(&attributes.fishable.to_le_bytes());
        bytes.extend_from_slice(&attributes.deep.to_le_bytes());
        for layer in &chunk.layers {
            append_client_layer(&mut bytes, layer);
        }
    }
    bytes
}

fn append_client_layer(bytes: &mut Vec<u8>, layer: &wow_adt::adt_tex::WaterLayer) {
    bytes.extend_from_slice(&layer.liquid_type.to_le_bytes());
    bytes.extend_from_slice(&layer.liquid_object.to_le_bytes());
    append_float(bytes, layer.min_height);
    append_float(bytes, layer.max_height);
    bytes.extend_from_slice(&[layer.x_offset, layer.y_offset, layer.width, layer.height]);
    bytes.extend_from_slice(&layer.exists);
    append_floats(bytes, &layer.vertex_heights);
    bytes.extend_from_slice(&(layer.vertex_uvs.len() as u32).to_le_bytes());
    for [u, v] in &layer.vertex_uvs {
        append_float(bytes, *u);
        append_float(bytes, *v);
    }
    append_blob(bytes, &layer.vertex_depths);
    append_blob(bytes, &layer.object_vertex_bytes);
}

pub(super) fn error(error: &str) -> Vec<u8> {
    let mut bytes = header(b"WMER", 1);
    append_blob(&mut bytes, error.as_bytes());
    bytes
}

/// Accessed raw record inventory for successful inputs, not an alternate parser oracle.
pub(super) fn raw_records(payload: &[u8]) -> Vec<u8> {
    let mut bytes = header(b"WMRW", 256);
    for index in 0..256 {
        let offset = index * 12;
        bytes.extend_from_slice(&payload[offset..offset + 12]);
        let instance = wow_chunk::read_u32(payload, offset).unwrap() as usize;
        let count = wow_chunk::read_u32(payload, offset + 4).unwrap() as usize;
        let attributes = wow_chunk::read_u32(payload, offset + 8).unwrap() as usize;
        if attributes != 0 {
            bytes.extend_from_slice(&payload[attributes..attributes + 16]);
        }
        if instance == 0 {
            continue;
        }
        for layer in 0..count {
            let start = instance + layer * 24;
            bytes.extend_from_slice(&payload[start..start + 24]);
        }
    }
    bytes
}

pub(super) fn water_bounds(bounds: &[(glam::Vec3, glam::Vec3)]) -> Vec<u8> {
    let mut bytes = header(b"WMBN", bounds.len());
    for (min, max) in bounds {
        for value in [min.x, min.y, min.z, max.x, max.y, max.z] {
            append_float(&mut bytes, value);
        }
    }
    assert_eq!(bytes.len(), 12 + 24 * bounds.len());
    bytes
}
