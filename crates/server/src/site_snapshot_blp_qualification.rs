mod blp_qualification {
    use super::super::{Bundle, catalog};
    use serde_json::{Value, json};
    use std::any::Any;
    use std::fs;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::path::Path;

    fn panic_text(payload: &dyn Any) -> &str {
        if let Some(message) = payload.downcast_ref::<String>() {
            message
        } else if let Some(message) = payload.downcast_ref::<&str>() {
            message
        } else {
            "non-string panic"
        }
    }

    fn capture(output: &Path, name: &str, input: &Path, id: Option<u32>) -> Value {
        let mut bundle = Bundle::new();
        match catch_unwind(AssertUnwindSafe(|| catalog::icon(&mut bundle, input, id))) {
            Ok(Ok(path)) => {
                let bytes = path.as_ref().map(|path| bundle[path].clone()).unwrap_or_default();
                if path.is_some() {
                    fs::write(output.join(format!("{name}.png")), &bytes).unwrap();
                    let rgba = image::load_from_memory(&bytes).unwrap().to_rgba8();
                    fs::write(output.join(format!("{name}.rgba.bin")), rgba.as_raw()).unwrap();
                    json!({"status": "ok", "path": path, "bytes": bytes.len(),
                        "width": rgba.width(), "height": rgba.height()})
                } else {
                    assert!(bundle.is_empty());
                    json!({"status": "none", "path": path})
                }
            }
            Ok(Err(error)) => json!({"status": "error", "message": error.to_string()}),
            Err(payload) => json!({"status": "panic", "message": panic_text(payload.as_ref())}),
        }
    }

    fn capture_icon_edges(output: &Path, input: &Path, id: u32) -> Vec<Value> {
        let png = fs::read(output.join("real_464811.png")).unwrap();
        let mut bundle = Bundle::new();
        let path = format!("icons/{id}.png");
        bundle.insert(path.clone(), png.clone());
        assert_eq!(catalog::icon(&mut bundle, input, Some(id)).unwrap(), Some(path.clone()));
        assert_eq!(bundle[&path], png);
        fs::write(output.join("cached_icon.png"), &png).unwrap();
        fs::write(input.join(format!("{id}.png")), &png).unwrap();
        fs::write(input.join(format!("{id}.blp")), b"invalid BLP").unwrap();
        let preferred = capture(output, "png_preferred", input, Some(id));
        assert_eq!(preferred["status"], "ok");
        assert_eq!(fs::read(output.join("png_preferred.png")).unwrap(), png);
        fs::write(input.join(format!("{id}.png")), b"invalid PNG").unwrap();
        let invalid_png = capture(output, "invalid_png", input, Some(id));
        assert_eq!(invalid_png["status"], "error");
        vec![
            json!({"name": "cached_icon", "icon": {"status": "ok", "path": path, "bytes": png.len()}}),
            json!({"name": "png_preferred", "icon": preferred}),
            json!({"name": "invalid_png", "icon": invalid_png}),
        ]
    }

    #[test]
    #[ignore = "qualification needs explicit frozen input manifest and new receipt directory"]
    fn capture_icon_before_only() {
        let input = std::env::var("WOW_BLP_INPUTS").expect("WOW_BLP_INPUTS");
        let output = std::env::var("WOW_BLP_ICON_OUTPUT").expect("WOW_BLP_ICON_OUTPUT");
        let output = Path::new(&output);
        fs::create_dir(output).expect("receipt directory must be new");
        let manifest: Value = serde_json::from_slice(&fs::read(input).unwrap()).unwrap();
        let cases = manifest["cases"].as_array().unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let mut receipts = Vec::new();
        for case in cases {
            let name = case["name"].as_str().unwrap();
            let id = u32::try_from(case["id"].as_u64().unwrap()).unwrap();
            let path = scratch.path().join(format!("{id}.blp"));
            std::os::unix::fs::symlink(case["path"].as_str().unwrap(), path).unwrap();
            let icon = capture(output, name, scratch.path(), Some(id));
            receipts.push(json!({"name": name, "icon": icon}));
        }
        let none = capture(output, "none_id", scratch.path(), None);
        let missing = capture(output, "missing_file", scratch.path(), Some(999_999_999));
        assert_eq!(none["status"], "none");
        assert_eq!(missing["status"], "none");
        receipts.push(json!({"name": "none_id", "icon": none}));
        receipts.push(json!({"name": "missing_file", "icon": missing}));
        receipts.extend(capture_icon_edges(output, scratch.path(), 999_999_998));
        assert_eq!(receipts.len(), cases.len() + 5);
        fs::write(output.join("outcomes.json"), serde_json::to_vec_pretty(&receipts).unwrap()).unwrap();
        println!("Captured {} independent icon outcomes", receipts.len());
    }
}
