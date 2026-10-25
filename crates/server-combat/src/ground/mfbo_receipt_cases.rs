pub(super) struct Case {
    pub name: String,
    pub bytes: Vec<u8>,
}

fn append_chunk(output: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
    output.extend_from_slice(tag);
    output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    output.extend_from_slice(payload);
}

fn replace_bounds(root: &[u8], payloads: &[&[u8]]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut replaced = 0;
    for chunk in wow_chunk::ChunkIter::new(root) {
        let (tag, payload) = chunk.unwrap();
        if tag == b"OBFM" {
            replaced += 1;
            for payload in payloads {
                append_chunk(&mut output, tag, payload);
            }
        } else {
            append_chunk(&mut output, tag, payload);
        }
    }
    assert_eq!(replaced, 1, "the frozen base root has one MFBO");
    output
}

fn stored_bounds(max: [i16; 9], min: [i16; 9]) -> Vec<u8> {
    max.into_iter()
        .chain(min)
        .flat_map(i16::to_le_bytes)
        .collect()
}

fn case(name: &str, bytes: Vec<u8>) -> Case {
    Case {
        name: name.into(),
        bytes,
    }
}

fn short_and_valid_cases(root: &[u8]) -> Vec<Case> {
    let original = wow_chunk::find_reversed_chunk(root, b"MFBO")
        .unwrap()
        .unwrap();
    assert_eq!(original.len(), 36);
    let mut cases: Vec<_> = (0..36)
        .map(|len| {
            case(
                &format!("short-{len:02}"),
                replace_bounds(root, &[&original[..len]]),
            )
        })
        .collect();
    cases.push(case("absent", replace_bounds(root, &[])));
    cases.push(case("exact-36", root.to_vec()));
    for tail in [1, 2] {
        let mut payload = original.to_vec();
        payload.extend_from_slice(&[0xde, 0xad][..tail]);
        cases.push(case(
            &format!("unused-tail-{tail}"),
            replace_bounds(root, &[&payload]),
        ));
    }
    let extreme = stored_bounds(
        [i16::MIN, i16::MAX, -1, 0, 1, 1700, 1800, -1237, -1000],
        [i16::MAX, i16::MIN, -1, 0, 1, 300, 1000, -1237, -2000],
    );
    cases.push(case("signed-extrema", replace_bounds(root, &[&extreme])));
    cases
}

fn duplicate_cases(root: &[u8]) -> Vec<Case> {
    let first = wow_chunk::find_reversed_chunk(root, b"MFBO")
        .unwrap()
        .unwrap();
    let last = stored_bounds(
        [2026; 9],
        [
            -1500, -1501, -1502, -1503, -1504, -1505, -1506, -1507, -1508,
        ],
    );
    vec![
        case("duplicate-distinct", replace_bounds(root, &[first, &last])),
        case(
            "duplicate-first-short",
            replace_bounds(root, &[&first[..19], &last]),
        ),
        case(
            "duplicate-last-short",
            replace_bounds(root, &[first, &last[..35]]),
        ),
    ]
}

fn truncated_tail(mut root: Vec<u8>) -> Vec<u8> {
    root.extend_from_slice(b"BAD!\x08\0\0\0x");
    root
}

fn truncated_prefix(root: &[u8]) -> Vec<u8> {
    let mut bytes = b"BAD!\xff\xff\xff\xffx".to_vec();
    bytes.extend_from_slice(root);
    bytes
}

fn short_first_mcvt(root: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut replaced = false;
    for chunk in wow_chunk::ChunkIter::new(root) {
        let (tag, payload) = chunk.unwrap();
        if tag == b"KNCM" && !replaced {
            let mut bad = payload[..128].to_vec();
            append_chunk(&mut bad, b"TVCM", &[0]);
            append_chunk(&mut output, tag, &bad);
            replaced = true;
        } else {
            append_chunk(&mut output, tag, payload);
        }
    }
    assert!(replaced);
    output
}

fn precedence_cases(root: &[u8]) -> Vec<Case> {
    let bounds = wow_chunk::find_reversed_chunk(root, b"MFBO")
        .unwrap()
        .unwrap();
    let short = replace_bounds(root, &[&bounds[..19]]);
    vec![
        case("valid-bounds-truncated-tail", truncated_tail(root.to_vec())),
        case("short-bounds-truncated-tail", truncated_tail(short.clone())),
        case(
            "absent-bounds-truncated-tail",
            truncated_tail(replace_bounds(root, &[])),
        ),
        case("truncated-prefix-valid-bounds", truncated_prefix(root)),
        case("truncated-prefix-short-bounds", truncated_prefix(&short)),
        case("valid-bounds-short-mcvt", short_first_mcvt(root)),
        case("short-bounds-short-mcvt", short_first_mcvt(&short)),
    ]
}

pub(super) fn definitions(root: &[u8]) -> Vec<Case> {
    let mut cases = short_and_valid_cases(root);
    cases.extend(duplicate_cases(root));
    cases.extend(precedence_cases(root));
    assert_eq!(cases.len(), 51);
    cases
}
