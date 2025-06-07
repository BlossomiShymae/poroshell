use std::{ iter::Peekable };

use derive_more::{ Display, From };
use serde::{ Deserialize, Serialize, de::Error as DeError };
use serde_json::Value;
use itertools::Itertools;

use crate::error::{ ParseError, SyntaxError };

/// A path to a property in the data structure.
/// - `.` is used to separate nested properties.
/// - `*` is used to match any property at that level.
/// - `0`, `1` etc. are used to access array elements.
/// - `"key"` is used to access object properties.
#[derive(Serialize, Deserialize, Debug, Display, Clone, PartialEq, Eq, Hash, From)]
pub struct DotPathStr<'a>(pub &'a str);

impl<'a, T: AsRef<str>> From<&'a T> for DotPathStr<'a> {
    fn from(path: &'a T) -> Self {
        DotPathStr(path.as_ref())
    }
}

impl DotPathStr<'_> {
    pub fn tokenize(&self) -> Result<Vec<DotToken>, SyntaxError> {
        self.0.split('.').map(parse_token).collect()
    }
}

fn parse_quoted_property(segment: &str) -> Option<DotToken<'_>> {
    if segment.starts_with('"') && segment.ends_with('"') {
        let inner = &segment[1..segment.len() - 1];
        Some(DotToken::Property(inner))
    } else {
        None
    }
}

fn parse_wild_once_or_until_next(token: &str) -> Option<DotToken<'_>> {
    match token {
        "**" => Some(DotToken::Wildcard(Wildcard::UntilNext)),
        "*" => Some(DotToken::Wildcard(Wildcard::Once)),
        _ => None,
    }
}

fn parse_token(token: &str) -> Result<DotToken<'_>, SyntaxError> {
    /* // Union first (cannot contain wildcards)
    if token.contains('|') {
        return token
            .split('|')
            .map(parse_union_member)
            .collect::<Result<Vec<_>, _>>()
            .map(DotToken::union);
    } */

    // Quoted property
    if let Some(quoted) = parse_quoted_property(token) {
        return Ok(quoted);
    }

    // Wildcard `*` or `**`
    if let Some(wildcard) = parse_wild_once_or_until_next(token) {
        return Ok(wildcard);
    }

    // Array index 0, 1, 2, etc.
    if let Ok(index) = token.parse::<usize>() {
        Ok(DotToken::Index(index))
    } else {
        // Naked property name
        Ok(DotToken::Property(token))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Wildcard {
    /// Single (`*`) wildcard that matches any property once.
    Once,
    /// Double (`**`) wildcard that matches any property on every level until the next known non-wild property.
    UntilNext,
}

impl std::fmt::Display for Wildcard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Wildcard::Once => write!(f, "*"),
            Wildcard::UntilNext => write!(f, "**"),
        }
    }
}

/// A type of token in a [DotPath].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DotToken<'a> {
    /// A property name in the data structure.
    Property(&'a str),
    /// An array index.
    Index(usize),
    /// `*` A wildcard that matches any property at that level.
    Wildcard(Wildcard),
}

impl std::fmt::Display for DotToken<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DotToken::Property(name) => write!(f, "{name}"),
            DotToken::Wildcard(wildcard) => wildcard.fmt(f),
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

impl<'a> TryFrom<&'a DotPathStr<'a>> for DotPathIterator<'a> {
    type Error = SyntaxError;

    fn try_from(dot_path: &'a DotPathStr<'a>) -> Result<Self, Self::Error> {
        dot_path.tokenize().map(|tokens| Self { tokens, index: 0 })
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

    fn navigate<'a>(
        &self,
        path: impl Into<DotPathStr<'a>>,
        in_wild: bool
    ) -> Result<Vec<&Self::Value>, Self::Error>;

    fn patch_mut<'a>(
        &mut self,
        path: impl Into<DotPathStr<'a>>,
        value: Option<Value>
    ) -> Result<(), Self::Error>;
}

impl Patch for Value {
    type Error = ParseError;
    type Value = Value;

    fn patch_mut<'a>(
        &mut self,
        path: impl Into<DotPathStr<'a>>,
        value: Option<Value>
    ) -> Result<(), Self::Error> {
        let mut current = self;
        let path: DotPathStr = path.into();
        let mut tokens = DotPathIterator::try_from(&path)?.peekable();

        while let Some(token) = tokens.next() {
            match token {
                DotToken::Property(prop) => {
                    // Check if the current value is an object
                    if let Value::Object(obj) = current {
                        // Exit when we reach the last token
                        if tokens.peek().is_none() {
                            // replace the value
                            if let Some(value) = value {
                                obj.insert(prop.to_string(), value);
                            } else {
                                // If the value is None, remove the property
                                obj.remove(prop);
                            }
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
                            if let Some(value) = value {
                                // If the index is out of bounds, extend the array
                                if index >= arr.len() {
                                    arr.resize_with(index + 1, || Value::Null);
                                }
                                // Set the value at the index
                                arr[index] = value;
                            } else {
                                // If the value is None, remove the element at the index
                                if index < arr.len() {
                                    arr.remove(index);
                                }
                            }
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
                DotToken::Wildcard(_wild) => {
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
                                ParseError::Json(
                                    serde_json::Error::custom(
                                        format!(
                                            "Wildcard expected an object or array at path {path} but found {current}"
                                        )
                                    )
                                )
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

    fn navigate<'a>(
        &self,
        path: impl Into<DotPathStr<'a>>,
        in_wild: bool
    ) -> Result<Vec<&Self::Value>, Self::Error> {
        let mut current = self;
        let path: DotPathStr = path.into();
        let mut tokens = DotPathIterator::try_from(&path)?.peekable();
        let mut result = Vec::new();

        while let Some(token) = tokens.next() {
            match token {
                DotToken::Property(prop) => {
                    if let Value::Object(obj) = current {
                        if let Some(next) = obj.get(prop) {
                            if tokens.peek().is_none() {
                                result.push(next);
                            }

                            current = next;
                        } else {
                            if in_wild {
                                continue;
                            }
                            return Err(
                                serde_json::Error
                                    ::custom(
                                        format!(
                                            "Property \"{prop}\" not found in path \"{path}\" for {current}"
                                        )
                                    )
                                    .into()
                            );
                        }
                    } else {
                        if in_wild {
                            continue;
                        }
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
                    if let Value::Array(arr) = current {
                        if index < arr.len() {
                            if tokens.peek().is_none() {
                                result.push(&arr[index]);
                            }
                            current = &arr[index];
                        } else {
                            if in_wild {
                                continue;
                            }
                            return Err(
                                serde_json::Error
                                    ::custom(format!("Index {index} not found at path {path}"))
                                    .into()
                            );
                        }
                    } else {
                        if in_wild {
                            continue;
                        }
                        return Err(
                            serde_json::Error
                                ::custom(
                                    format!("Expected an array at path {path}, found {current}")
                                )
                                .into()
                        );
                    }
                }
                DotToken::Wildcard(wild) => {
                    match (wild, current) {
                        (Wildcard::Once, Value::Object(a)) => {
                            let sub_path = tokens.clone().join(".");
                            let sub_path = DotPathStr(&sub_path);
                            for v in a.values() {
                                if let Ok(sub_result) = v.navigate(sub_path.clone(), true) {
                                    result.extend(sub_result);
                                }
                            }
                        }
                        (Wildcard::Once, Value::Array(a)) => {
                            let sub_path = tokens.clone().join(".");
                            let sub_path = DotPathStr(&sub_path);
                            for v in a.iter() {
                                if let Ok(sub_result) = v.navigate(sub_path.clone(), true) {
                                    result.extend(sub_result);
                                };
                            }
                        }
                        (Wildcard::UntilNext, Value::Object(a)) => {
                            traverse_until_next(current, &mut tokens, &mut result, a.values())?;
                        }
                        (Wildcard::UntilNext, Value::Array(a)) => {
                            traverse_until_next(current, &mut tokens, &mut result, a.iter())?;
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
                }
            }
        }

        Ok(result)
    }
}

fn traverse_until_next<'a>(
    current: &'a Value,
    tokens: &mut Peekable<DotPathIterator<'_>>,
    result: &mut Vec<&'a Value>,
    values: impl IntoIterator<Item = &'a Value>
) -> Result<(), <Value as Patch>::Error> {
    let next_token = tokens.peek().cloned();
    match next_token {
        Some(next_token) => {
            traverse_until_next_helper(current, &next_token, tokens.clone(), result)?;
        }
        None => {
            // If no next token, treat ** as recursive wildcard collecting all descendants
            let sub_path = tokens.clone().join(".");
            for v in values {
                let sub_path = DotPathStr(&sub_path);
                let sub_result = v.navigate(sub_path, true)?;
                result.extend(sub_result);
            }
        }
    }
    Ok(())
}

fn traverse_until_next_helper<'a>(
    current: &'a Value,
    next_token: &DotToken,
    mut tokens: Peekable<DotPathIterator>,
    result: &mut Vec<&'a Value>
) -> Result<(), ParseError> {
    match current {
        Value::Object(obj) => {
            // Check if next_token matches any key here.
            let next_value = match next_token {
                DotToken::Property(prop) => obj.get(*prop),
                DotToken::Index(index) => obj.get(&index.to_string()),
                _ => None,
            };
            if let Some(value) = next_value {
                // Advance the iterator because we found a match
                tokens.next();

                // If there are no more tokens, add the value to the result
                if tokens.peek().is_none() {
                    result.push(value);
                } else {
                    // Otherwise, recurse into the value
                    traverse_until_next_helper(value, next_token, tokens.clone(), result)?;
                }
            }

            // No match, recurse into all values
            for v in obj.values() {
                traverse_until_next_helper(v, next_token, tokens.clone(), result)?;
            }
        }
        Value::Array(arr) => {
            // Check if next_token matches any index here.
            if let Some(value) = next_token.as_index().and_then(|index| arr.get(*index)) {
                // Advance the iterator because we found a match
                tokens.next();

                // If there are no more tokens, add the value to the result
                if tokens.peek().is_none() {
                    result.push(value);
                } else {
                    // Otherwise, recurse into the value
                    traverse_until_next_helper(value, next_token, tokens.clone(), result)?;
                }
            }

            // No match, recurse into all values
            for v in arr.iter() {
                traverse_until_next_helper(v, next_token, tokens.clone(), result)?;
            }
        }
        _ => {}
    }

    Ok(())
}

impl<'a> DotToken<'a> {
    /// Extend a start token into a full path.
    ///
    /// This token (`self`) is inserted at the start of the path. This is useful when
    /// you need to prepend a token to an existing path for navigation or patching.
    #[inline]
    pub fn prepend_to(self, path: impl Iterator<Item = Self>) -> String {
        std::iter::once(self).chain(path).join(".")
    }

    pub fn as_index(&self) -> Option<&usize> {
        if let Self::Index(v) = self { Some(v) } else { None }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_dot_path() -> Result<(), SyntaxError> {
        let path = DotPathStr("a.1.*.d");
        let tokens = path.tokenize()?;
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0], DotToken::Property("a"));
        assert_eq!(tokens[1], DotToken::Index(1));
        assert_eq!(tokens[2], DotToken::Wildcard(Wildcard::Once));
        assert_eq!(tokens[3], DotToken::Property("d"));

        Ok(())
    }

    #[test]
    fn test_dot_path_iterator() -> Result<(), SyntaxError> {
        let path = DotPathStr("a.1.**.d");
        let mut iter = DotPathIterator::try_from(&path)?;
        assert_eq!(iter.next(), Some(DotToken::Property("a")));
        assert_eq!(iter.next(), Some(DotToken::Index(1)));
        assert_eq!(iter.next(), Some(DotToken::Wildcard(Wildcard::UntilNext)));
        assert_eq!(iter.next(), Some(DotToken::Property("d")));
        assert_eq!(iter.next(), None);

        Ok(())
    }

    #[test]
    fn test_dot_path_display() -> Result<(), SyntaxError> {
        let path = DotPathStr("a.1.*.d");
        let iter = DotPathIterator::try_from(&path)?;
        let display = iter.to_string();
        assert_eq!(display, "a.1.*.d");

        Ok(())
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
        json.patch_mut(path, Some(value)).unwrap();

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
    fn test_dot_path_patch_create_new() -> Result<(), ParseError> {
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
        let result = json.patch_mut(path, Some(value))?;
        assert_eq!(
            json,
            serde_json::json!({
                "a": {
                    "b": [
                        {"c": 1, "d": 2, "x": 200},
                        {"d": 2, "x": 200}
                    ],
                    "e": 3
                },
                "f": [
                    {"g": 4},
                    {"h": 5}
                ]
            })
        );
        Ok(result)
    }

    #[test]
    fn test_get() {
        let json =
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

        let path = DotPathStr("a.b.0.d");
        let values = json.navigate(path, false).unwrap();
        println!("Values: {:?}", values);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], &serde_json::json!(2));
    }

    #[test]
    fn test_wilds() {
        let json =
            serde_json::json!({
                "a": {
                    "org": {
                        "groups": [
                            {
                                "info": {
                                    "name": "Org Group 0"
                                }
                            },
                            {
                                "info": {
                                    "name": "Org Group 1"
                                }
                            }
                        ]
                    }
                },
                "b": {
                    "com": {
                        "channels": [
                            {
                                "group": {
                                    "info": {
                                        "name": "Com Channel 0 Group Name"
                                    }
                                },
                                "info": {
                                    "name": "Com Channel 0"
                                }
                            },
                            {
                                "info": {
                                    "name": "Com Group 1"
                                }
                            }
                        ]
                    }
                }
            });

        let path = DotPathStr("**.name");
        let values = json.navigate(path, true).unwrap();
        assert_eq!(values.len(), 5);
        assert_eq!(values[0], &serde_json::json!("Org Group 0"));
        assert_eq!(values[1], &serde_json::json!("Org Group 1"));
        assert_eq!(values[2], &serde_json::json!("Com Channel 0 Group Name"));
        assert_eq!(values[3], &serde_json::json!("Com Channel 0"));
        assert_eq!(values[4], &serde_json::json!("Com Group 1"));
    }
}
