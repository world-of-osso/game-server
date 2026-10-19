pub(super) struct Case {
    pub name: &'static str,
    pub bytes: Vec<u8>,
    pub expected_error: Option<String>,
}

fn chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = tag.to_vec();
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

fn mcnk_header() -> Vec<u8> {
    let mut bytes = vec![0; 128];
    bytes[0x34..0x38].copy_from_slice(&77u32.to_le_bytes());
    for (offset, value) in [(0x68, 12345f32), (0x6c, -543.25), (0x70, 18.5)] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn height_values(value: f32) -> Vec<u8> {
    (0..145)
        .flat_map(|index| (value + index as f32 * 0.25).to_le_bytes())
        .collect()
}

fn mcnk(value: f32) -> Vec<u8> {
    let mut bytes = mcnk_header();
    bytes.extend(chunk(b"TVCM", &height_values(value)));
    bytes
}

fn error(name: &'static str, bytes: Vec<u8>, expected: &str) -> Case {
    Case {
        name,
        bytes,
        expected_error: Some(expected.into()),
    }
}

fn parsed(name: &'static str, bytes: Vec<u8>) -> Case {
    Case {
        name,
        bytes,
        expected_error: None,
    }
}

fn build_short_input_cases() -> Vec<Case> {
    let mut short_height = mcnk_header();
    short_height.extend(chunk(b"TVCM", &[0; 579]));
    vec![
        error("empty-root", Vec::new(), "No KNCM (MCNK) chunks found"),
        error(
            "short-header",
            chunk(b"KNCM", &[0; 127]),
            "MCNK too small: 127 bytes",
        ),
        error(
            "missing-height",
            chunk(b"KNCM", &mcnk_header()),
            "MCNK missing TVCM sub-chunk",
        ),
        error(
            "short-height",
            chunk(b"KNCM", &short_height),
            "MCVT too small: 579 bytes (need 580)",
        ),
        error(
            "truncated-root",
            b"KNCM\x80\0\0\0x".to_vec(),
            "Chunk truncated at 0x0",
        ),
    ]
}

fn build_selected_height_cases() -> Vec<Case> {
    let mut before = mcnk_header();
    before.extend_from_slice(b"BAD!\0\x10\0\0xx");
    before.extend(chunk(b"TVCM", &height_values(1.0)));
    let mut after = mcnk(1.0);
    after.extend_from_slice(b"BAD!\0\x10\0\0xx");
    let mut duplicate = mcnk(1.0);
    duplicate.extend(chunk(b"TVCM", &height_values(999.0)));
    let mut values = height_values(2.0);
    values.extend_from_slice(&999f32.to_le_bytes());
    let mut extra = mcnk_header();
    extra.extend(chunk(b"TVCM", &values));
    vec![
        error(
            "truncated-subchunk-before-height",
            chunk(b"KNCM", &before),
            "MCNK missing TVCM sub-chunk",
        ),
        parsed("truncated-subchunk-after-height", chunk(b"KNCM", &after)),
        parsed("first-duplicate-height", chunk(b"KNCM", &duplicate)),
        parsed("unused-height-tail", chunk(b"KNCM", &extra)),
    ]
}

fn build_hole_and_order_cases() -> Vec<Case> {
    let mut low = mcnk(1.0);
    low[0x3c..0x3e].copy_from_slice(&(1u16 << 5).to_le_bytes());
    let mut high = mcnk(1.0);
    high[..4].copy_from_slice(&0x10000u32.to_le_bytes());
    high[0x14..0x1c].copy_from_slice(&0x8000_0000_0000_c0c0u64.to_le_bytes());
    high[0x3c..0x3e].copy_from_slice(&u16::MAX.to_le_bytes());
    let mut no_render = mcnk(1.0);
    no_render[..4].copy_from_slice(&0x41u32.to_le_bytes());
    let mut ordered = chunk(b"KNCM", &mcnk(2.0));
    let mut second = mcnk(7.0);
    second[0x34..0x38].copy_from_slice(&88u32.to_le_bytes());
    second[0x68..0x6c].copy_from_slice(&100f32.to_le_bytes());
    ordered.extend(chunk(b"KNCM", &second));
    vec![
        parsed("low-hole-mask", chunk(b"KNCM", &low)),
        parsed("high-hole-mask", chunk(b"KNCM", &high)),
        parsed("no-optional-render-payloads", chunk(b"KNCM", &no_render)),
        parsed("source-chunk-order", ordered),
    ]
}

fn build_failure_precedence_cases() -> Vec<Case> {
    let water = chunk(b"O2HM", &[0]);
    let mut water_before_height = chunk(b"KNCM", &mcnk_header());
    water_before_height.extend_from_slice(&water);
    let mut water_before_header = chunk(b"KNCM", &[0; 127]);
    water_before_header.extend_from_slice(&water);
    let offset = water_before_height.len();
    let mut root_failure = water_before_height.clone();
    root_failure.extend_from_slice(b"BAD!\x08\0\0\0xx");
    vec![
        error(
            "water-before-missing-height",
            water_before_height,
            "MH2O too small: 1 bytes",
        ),
        error(
            "water-before-short-header",
            water_before_header,
            "MH2O too small: 1 bytes",
        ),
        error(
            "root-before-water-and-height",
            root_failure,
            &format!("Chunk truncated at {offset:#x}"),
        ),
        error(
            "missing-chunks-before-water",
            water,
            "No KNCM (MCNK) chunks found",
        ),
    ]
}

pub(super) fn definitions() -> Vec<Case> {
    [
        build_short_input_cases(),
        build_selected_height_cases(),
        build_hole_and_order_cases(),
        build_failure_precedence_cases(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
