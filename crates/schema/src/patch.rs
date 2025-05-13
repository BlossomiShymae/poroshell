use derive_more::{ Display, From };
use serde::{ Deserialize, Serialize, de::Error as DeError };
use serde_json::Value;
use itertools::Itertools;

use crate::error::Error;

/// A path to a property in the data structure.
/// - `.` is used to separate nested properties.
/// - `*` is used to match any property at that level.
/// - `0`, `1` etc. are used to access array elements.
/// - `"key"` is used to access object properties.
#[derive(Serialize, Deserialize, Debug, Display, Clone, PartialEq, Eq, Hash, From)]
pub struct DotPathStr<'a>(pub &'a str);

impl DotPathStr<'_> {
    pub fn tokenize(&self) -> Vec<DotToken> {
        self.0
            .split('.')
            .map(|s| {
                match s {
                    "*" => DotToken::Wildcard,
                    _ => if let Ok(index) = s.parse::<usize>() {
                        DotToken::Index(index)
                    } else {
                        DotToken::Property(s)
                    }
                }
            })
            .collect()
    }
}

/// A type of token in a [DotPath].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DotToken<'a> {
    /// A property name in the data structure.
    Property(&'a str),
    /// A wildcard that matches any property at that level.
    Wildcard,
    /// An array index.
    Index(usize),
}

impl std::fmt::Display for DotToken<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DotToken::Property(name) => write!(f, "{name}"),
            DotToken::Wildcard => write!(f, "*"),
            DotToken::Index(index) => write!(f, "{index}"),
        }
    }
}

/// An iterator over [`DotToken`]s.
#[derive(Clone)]
pub struct DotPathIterator<'a> {
    tokens: Vec<DotToken<'a>>,
    index: usize,
}

impl<'a> DotPathIterator<'a> {
    pub fn new(dot_path: &'a DotPathStr) -> Self {
        Self { tokens: dot_path.tokenize(), index: 0 }
    }
}

impl<'a> Iterator for DotPathIterator<'a> {
    type Item = DotToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.tokens.len() {
            let token = self.tokens[self.index].clone();
            self.index += 1;
            Some(token)
        } else {
            None
        }
    }
}

impl std::fmt::Display for DotPathIterator<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, token) in self.tokens.iter().enumerate() {
            if i > 0 {
                write!(f, ".")?;
            }
            write!(f, "{token}")?;
        }
        Ok(())
    }
}

/// A trait for patching a data structure with a value at a given path.
pub trait Patch {
    type Value;
    type Error;

    fn patch_mut<'a>(
        &mut self,
        path: impl Into<DotPathStr<'a>>,
        value: Value
    ) -> Result<(), Self::Error>;
}

impl Patch for Value {
    type Error = Error;
    type Value = Value;

    fn patch_mut<'a>(
        &mut self,
        path: impl Into<DotPathStr<'a>>,
        value: Value
    ) -> Result<(), Self::Error> {
        let mut current = self;
        let path: DotPathStr = path.into();
        let mut tokens = DotPathIterator::new(&path).peekable();

        while let Some(token) = tokens.next() {
            match token {
                DotToken::Property(prop) => {
                    // Check if the current value is an object
                    if let Value::Object(obj) = current {
                        // Exit when we reach the last token
                        if tokens.peek().is_none() {
                            // replace the value
                            obj.insert(prop.to_string(), value);
                            return Ok(());
                        } else {
                            // Dig deeper

                            // If the property doesn't exist, create it as an empty object
                            if !obj.contains_key(prop) {
                                obj.insert(prop.to_string(), Value::Object(serde_json::Map::new()));
                            }

                            current = obj.get_mut(prop).unwrap();
                        }
                        // The token was expecting this to be an object, but it wasn't.
                    } else {
                        return Err(
                            serde_json::Error
                                ::custom(
                                    format!("Expected an object at path {path}, found {current}")
                                )
                                .into()
                        );
                    }
                }
                DotToken::Index(index) => {
                    // Check if the current value is an array
                    if let Value::Array(arr) = current {
                        // Check if we're at the last token
                        if tokens.peek().is_none() {
                            // If so, set the value
                            arr[index] = value;
                            return Ok(());
                        } else {
                            // Otherwise, dig deeper
                            current = arr
                                .get_mut(index)
                                .ok_or_else(||
                                    serde_json::Error::custom(
                                        format!("Index {index} not found at path {path}")
                                    )
                                )?;
                        }
                    } else {
                        return Err(
                            serde_json::Error
                                ::custom(
                                    format!("Expected an array at path {path}, found {current}")
                                )
                                .into()
                        );
                    }
                }
                DotToken::Wildcard => {
                    match current {
                        Value::Object(obj) => {
                            for v in obj.values_mut() {
                                let sub_path = tokens.clone().join(".");
                                let sub_path = DotPathStr(&sub_path);
                                v.patch_mut(sub_path, value.clone())?;
                            }
                        }
                        Value::Array(arr) => {
                            for v in arr.iter_mut() {
                                let sub_path = tokens.clone().join(".");
                                let sub_path = DotPathStr(&sub_path);
                                v.patch_mut(sub_path, value.clone())?;
                            }
                        }
                        _ => {
                            return Err(
                                serde_json::Error
                                    ::custom(
                                        format!(
                                            "Wildcard expected an object or array at path {path} but found {current}"
                                        )
                                    )
                                    .into()
                            );
                        }
                    }
                    // Prevent the outer loop from continuing since the wildcard branches out
                    return Ok(());
                }
            }
        }

        Err(
            serde_json::Error
                ::custom(format!("Failed to patch value at path {path}: no terminal target"))
                .into()
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_dot_path() {
        let path = DotPathStr("a.1.*.d");
        let tokens = path.tokenize();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0], DotToken::Property("a"));
        assert_eq!(tokens[1], DotToken::Index(1));
        assert_eq!(tokens[2], DotToken::Wildcard);
        assert_eq!(tokens[3], DotToken::Property("d"));
    }

    #[test]
    fn test_dot_path_iterator() {
        let path = DotPathStr("a.1.*.d");
        let mut iter = DotPathIterator::new(&path);
        assert_eq!(iter.next(), Some(DotToken::Property("a")));
        assert_eq!(iter.next(), Some(DotToken::Index(1)));
        assert_eq!(iter.next(), Some(DotToken::Wildcard));
        assert_eq!(iter.next(), Some(DotToken::Property("d")));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_dot_path_display() {
        let path = DotPathStr("a.1.*.d");
        let iter = DotPathIterator::new(&path);
        let display = iter.to_string();
        assert_eq!(display, "a.1.*.d");
    }

    #[test]
    fn test_patch() {
        let mut json =
            serde_json::json!({
            "a": {
                "b": [
                    {"c": 1, "d": 2},
                    {"d": 2}
                ],
                "e": 3
            },
            "f": [
                {"g": 4},
                {"h": 5}
            ]
        });

        let path = DotPathStr("a.b.*.d");
        let value = serde_json::json!(200);
        json.patch_mut(path, value).unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "a": {
                    "b": [
                        {"c": 1, "d": 200},
                        {"d": 200}
                    ],
                    "e": 3
                },
                "f": [
                    {"g": 4},
                    {"h": 5}
                ]
        })
        );
    }

    #[test]
    fn test_dot_path_patch_error() {
        let mut json =
            serde_json::json!({
            "a": {
                "b": [
                    {"c": 1, "d": 2},
                    {"d": 2}
                ],
                "e": 3
            },
            "f": [
                {"g": 4},
                {"h": 5}
            ]
        });

        let path = DotPathStr("a.b.*.x");
        let value = serde_json::json!(200);
        let result = json.patch_mut(path, value);
        assert!(result.is_err());
    }
}
