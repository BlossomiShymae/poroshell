use serde::Deserialize;

/// Converts null, empty strings, and "null" to None, and a non-empty string to Some(string).
/// This is useful for deserializing optional fields that still get sent as empty strings.
pub fn deserialize_option_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where D: serde::Deserializer<'de>
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s.as_deref() {
        None | Some("") | Some("null") => Ok(None),
        Some(s) => Ok(Some(s.to_owned())),
    }
}

/// Coerces a value to a boolean. Handles "broken" JSON values like
/// `true`, `false`, `1`, `0`, `yes`, `no`, etc.
pub(crate) fn deserialize_bool_coerced<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where D: serde::Deserializer<'de>
{
    use serde_json::Value;
    use serde::de::{ Error, Unexpected };

    match Value::deserialize(deserializer)? {
        Value::Bool(b) => Ok(b),
        Value::Number(n) => Ok(n.as_i64().is_some_and(|i| i != 0)),
        Value::String(s) => {
            match s.trim().to_lowercase().as_str() {
                "true" | "1" | "yes" | "y" | "on" => Ok(true),
                "" | "false" | "0" | "no" | "n" | "off" => Ok(false),
                s =>
                    Err(
                        Error::invalid_value(
                            Unexpected::Str(s),
                            &"a recognized boolean value (.e.g true/false)"
                        )
                    ),
            }
        }
        v =>
            Err(
                Error::invalid_type(
                    Unexpected::Other(&format!("{v:?}")),
                    &"a bool, number, or boolean-like string"
                )
            ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn test_deserialize_option_string() {
        #[derive(Deserialize)]
        struct Test {
            #[serde(default, deserialize_with = "deserialize_option_string")]
            field: Option<String>,
        }

        let json = r#"{}"#;
        let result: Test = serde_json::from_str(json).unwrap();
        assert_eq!(result.field, None);

        let json = r#"{"field": null}"#;
        let result: Test = serde_json::from_str(json).unwrap();
        assert_eq!(result.field, None);

        let json = r#"{"field": ""}"#;
        let result: Test = serde_json::from_str(json).unwrap();
        assert_eq!(result.field, None);

        let json = r#"{"field": "null"}"#;
        let result: Test = serde_json::from_str(json).unwrap();
        assert_eq!(result.field, None);

        let json = r#"{"field": "hello"}"#;
        let result: Test = serde_json::from_str(json).unwrap();
        assert_eq!(result.field, Some("hello".to_string()));
    }
}
