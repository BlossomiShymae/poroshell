use std::collections::BTreeMap;

use serde::ser::{ Serialize, Serializer };
use fxhash::FxHashMap as HashMap;

/// Serializes a HashMap by converting it to a BTreeMap.
/// This ensures that the keys are sorted in a consistent order.
/// This is useful for generating deterministic JSON output.
pub fn serialize_as_btree_map<S, T>(
    value: &HashMap<String, T>,
    serializer: S
)
    -> Result<S::Ok, S::Error>
    where S: Serializer, T: Serialize
{
    let btree = BTreeMap::from_iter(value.iter());
    btree.serialize(serializer)
}

/// Returns true if an [`Option<String>`] is None or an empty string.
///
/// Use `#[serde(skip_serializing_if = "option_string_is_none_or_empty")]`
/// to skip serializing the field if it is None or an empty string.
///
/// See [`deserialize_option_string`] for the reverse operation.
///
/// [`deserialize_option_string`]: crate::serde::de::deserialize_option_string
pub fn option_string_is_none_or_empty(value: &Option<String>) -> bool {
    match value {
        Some(s) => s.is_empty(),
        None => true,
    }
}

/// Sorts a sequence of strings and serializes it.
pub fn serialize_strings_sorted<S: serde::Serializer>(
    value: &[String],
    serializer: S
) -> Result<S::Ok, S::Error> {
    let mut sorted = value.to_vec();
    sorted.sort();
    sorted.serialize(serializer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use derive_more::{ Deref, DerefMut };
    use ::serde::{ Deserialize, Serialize };

    #[test]
    fn test_serialize_ordered_map() {
        #[derive(Default, Deref, DerefMut, Serialize, Deserialize)]
        #[serde(transparent)]
        struct Test {
            #[serde(serialize_with = "serialize_as_btree_map")]
            map: HashMap<String, i32>,
        }

        let mut map = Test::default();
        map.insert("b".to_string(), 2);
        map.insert("a".to_string(), 1);

        let serialized = serde_json::to_string(&map).unwrap();
        assert_eq!(serialized, r#"{"a":1,"b":2}"#);
    }

    #[test]
    fn test_serialize_strings_sorted() {
        #[derive(Serialize)]
        #[serde(transparent)]
        struct Test {
            #[serde(serialize_with = "serialize_strings_sorted")]
            strings: Vec<String>,
        }
        let test = Test {
            strings: vec!["banana".to_string(), "apple".to_string()],
        };
        let serialized = serde_json::to_string(&test).unwrap();

        assert_eq!(serialized, r#"["apple","banana"]"#);
    }
}
