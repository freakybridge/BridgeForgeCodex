use super::*;

#[test]
fn strict_json_preserves_shapes_numbers_order_and_unrelated_keys() {
    for payload in [
        "null",
        "true",
        "false",
        "-42",
        "18446744073709551615",
        "1.25",
        r#""unicode: 中文 \u0041""#,
        r#"[null,false,42,{"x":1},{"x":2}]"#,
    ] {
        let strict = parse_unique_json(payload.as_bytes(), "sample").unwrap();
        let expected: Value = serde_json::from_slice(payload.as_bytes()).unwrap();
        assert_eq!(strict, expected, "{payload}");
    }
    let value = parse_unique_json(br#"{"second":2,"first":1}"#, "order").unwrap();
    assert_eq!(
        value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["second", "first"]
    );
    assert_eq!(
        serde_json::to_string(&value).unwrap(),
        r#"{"second":2,"first":1}"#
    );
}

#[test]
fn strict_json_rejects_duplicate_keys_at_every_depth_with_original_errors() {
    for (payload, expected) in [
        (
            r#"{"key":1,"key":2}"#,
            "sample is invalid JSON: duplicate JSON key: key at line 1 column 17",
        ),
        (
            r#"{"nested":{"key":1,"key":2}}"#,
            "sample is invalid JSON: duplicate JSON key: key at line 1 column 27",
        ),
        (
            r#"[{"key":1,"key":2}]"#,
            "sample is invalid JSON: duplicate JSON key: key at line 1 column 18",
        ),
        (
            r#"{"a":1,"\u0061":2}"#,
            "sample is invalid JSON: duplicate JSON key: a at line 1 column 18",
        ),
    ] {
        assert_eq!(
            parse_unique_json(payload.as_bytes(), "sample").unwrap_err(),
            expected
        );
    }
}

#[test]
fn strict_json_keeps_invalid_input_error_labels_and_serde_positions() {
    for payload in [
        b"{".as_slice(),
        b"[] trailing",
        b"NaN",
        b"{\"a\":}",
        &[b'"', 0xff, b'"'],
    ] {
        let expected = serde_json::from_slice::<Value>(payload)
            .unwrap_err()
            .to_string();
        assert_eq!(
            parse_unique_json(payload, "cache").unwrap_err(),
            format!("cache is invalid JSON: {expected}")
        );
    }
    assert_eq!(
        parse_unique_json(b" 42 \n\t", "sample").unwrap(),
        Value::from(42)
    );
}
