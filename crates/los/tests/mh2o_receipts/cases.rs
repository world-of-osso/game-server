pub(super) struct Case {
    pub name: String,
    pub payload: Vec<u8>,
}

const DIRECTORY_BYTES: usize = 256 * 12;
const INSTANCE_OFFSET: usize = DIRECTORY_BYTES;
const ATTRIBUTE_OFFSET: usize = INSTANCE_OFFSET + 24;
const BITMAP_OFFSET: usize = ATTRIBUTE_OFFSET + 16;
const VERTEX_OFFSET: usize = BITMAP_OFFSET + 8;

fn word(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn layer(lvf: u16, stride: usize) -> Vec<u8> {
    // A3x2 rectangle has12 vertices;8 bitmap bytes satisfy both existing layouts.
    let mut bytes = vec![0; VERTEX_OFFSET + 12 * stride];
    word(&mut bytes, 0, INSTANCE_OFFSET as u32);
    word(&mut bytes, 4, 1);
    word(&mut bytes, 8, ATTRIBUTE_OFFSET as u32);
    bytes[INSTANCE_OFFSET..INSTANCE_OFFSET + 2].copy_from_slice(&5u16.to_le_bytes());
    bytes[INSTANCE_OFFSET + 2..INSTANCE_OFFSET + 4].copy_from_slice(&lvf.to_le_bytes());
    word(&mut bytes, INSTANCE_OFFSET + 4, (-0.0f32).to_bits());
    word(&mut bytes, INSTANCE_OFFSET + 8, 99.5f32.to_bits());
    bytes[INSTANCE_OFFSET + 12..INSTANCE_OFFSET + 16].copy_from_slice(&[2, 1, 3, 2]);
    word(&mut bytes, INSTANCE_OFFSET + 16, BITMAP_OFFSET as u32);
    word(&mut bytes, INSTANCE_OFFSET + 20, VERTEX_OFFSET as u32);
    bytes[ATTRIBUTE_OFFSET..ATTRIBUTE_OFFSET + 8]
        .copy_from_slice(&0x0123_4567_89ab_cdefu64.to_le_bytes());
    bytes[ATTRIBUTE_OFFSET + 8..ATTRIBUTE_OFFSET + 16]
        .copy_from_slice(&0x8000_0000_0000_0001u64.to_le_bytes());
    bytes[BITMAP_OFFSET..BITMAP_OFFSET + 8].copy_from_slice(&[
        0b0010_1101,
        0b0000_0101,
        0,
        0,
        0,
        0,
        0,
        0,
    ]);
    // Concrete bytes expose sequential-array versus strided-height interpretation.
    for (index, slot) in bytes[VERTEX_OFFSET..].chunks_mut(4).enumerate() {
        let value = (index as f32 * 0.25 + 2.0).to_le_bytes();
        slot.copy_from_slice(&value[..slot.len()]);
    }
    bytes
}

fn case(name: &str, payload: Vec<u8>) -> Case {
    Case {
        name: name.into(),
        payload,
    }
}

fn layout_cases() -> Vec<Case> {
    let mut result: Vec<_> = [(0, 5), (1, 8), (2, 1), (3, 9), (4, 5)]
        .into_iter()
        .map(|(lvf, stride)| case(&format!("lvf-{lvf}"), layer(lvf, stride)))
        .collect();
    result.extend(
        [1, 5, 8, 9, 6]
            .into_iter()
            .map(|stride| case(&format!("object-42-stride-{stride}"), layer(42, stride))),
    );
    let mut no_bitmap = layer(0, 5);
    word(&mut no_bitmap, INSTANCE_OFFSET + 16, 0);
    result.push(case("absent-bitmap", no_bitmap));
    let mut no_vertices = layer(0, 5);
    word(&mut no_vertices, INSTANCE_OFFSET + 20, 0);
    result.push(case("absent-vertices", no_vertices));
    let mut alias = layer(0, 5);
    word(&mut alias, INSTANCE_OFFSET + 16, VERTEX_OFFSET as u32);
    result.push(case("bitmap-vertex-alias", alias));
    let mut padded = layer(42, 5);
    padded.extend_from_slice(&[0; 12]);
    result.push(case("object-block-padding", padded));
    result
}

fn truncation_cases() -> Vec<Case> {
    let mut bitmap = layer(0, 5);
    let end = bitmap.len() as u32;
    word(&mut bitmap, INSTANCE_OFFSET + 16, end + 8);
    let mut vertex = layer(0, 5);
    word(&mut vertex, INSTANCE_OFFSET + 20, end + 8);
    let mut attributes = layer(0, 5);
    word(&mut attributes, 8, end - 15);
    let mut instance = layer(0, 5);
    word(&mut instance, 0, end - 23);
    vec![
        case("short-directory", vec![0; DIRECTORY_BYTES - 1]),
        case("short-instance", instance),
        case("short-attributes", attributes),
        case("bitmap-outside-payload", bitmap),
        case("vertex-outside-payload", vertex),
    ]
}

fn empty_and_precedence_cases() -> Vec<Case> {
    let mut zero_offset = vec![0; DIRECTORY_BYTES];
    word(&mut zero_offset, 4, 1);
    let mut zero_count = vec![0; DIRECTORY_BYTES];
    word(&mut zero_count, 0, u32::MAX);
    let mut combined = layer(0, 5);
    let end = combined.len() as u32;
    word(&mut combined, 8, end - 15);
    word(&mut combined, 12, end - 23);
    word(&mut combined, 16, 1);
    let mut attributes_without_layers = vec![0; DIRECTORY_BYTES];
    word(&mut attributes_without_layers, 8, DIRECTORY_BYTES as u32);
    vec![
        case("empty-directory", vec![0; DIRECTORY_BYTES]),
        case("zero-instance-positive-count", zero_offset),
        case("unused-instance-offset", zero_count),
        case("bad-attributes-without-layers", attributes_without_layers),
        case("attributes-before-later-instance-failure", combined),
    ]
}

fn multiple_layers() -> Case {
    let mut bytes = layer(0, 5);
    let second = bytes.len();
    let descriptor = bytes[INSTANCE_OFFSET..INSTANCE_OFFSET + 24].to_vec();
    // Descriptors must be consecutive. Insert the second descriptor before attributes.
    bytes.splice(ATTRIBUTE_OFFSET..ATTRIBUTE_OFFSET, descriptor);
    word(&mut bytes, 4, 2);
    word(&mut bytes, 8, (ATTRIBUTE_OFFSET + 24) as u32);
    for offset in [INSTANCE_OFFSET, INSTANCE_OFFSET + 24] {
        word(&mut bytes, offset + 16, (BITMAP_OFFSET + 24) as u32);
        word(&mut bytes, offset + 20, (VERTEX_OFFSET + 24) as u32);
    }
    bytes[INSTANCE_OFFSET + 24..INSTANCE_OFFSET + 26].copy_from_slice(&7u16.to_le_bytes());
    word(&mut bytes, INSTANCE_OFFSET + 24 + 4, 88.0f32.to_bits());
    word(&mut bytes, INSTANCE_OFFSET + 24 + 20, 0);
    assert_eq!(bytes.len(), second + 24);
    case("two-ordered-layers", bytes)
}

pub(super) fn definitions() -> Vec<Case> {
    let mut cases = layout_cases();
    cases.extend(truncation_cases());
    cases.extend(empty_and_precedence_cases());
    cases.push(multiple_layers());
    cases
}
