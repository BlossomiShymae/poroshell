use std::str::FromStr;

use derive_more::Display;
use fxhash::{ FxHashMap, FxHashSet };
use serde::{
    de::{ Deserializer, Visitor },
    ser::{ SerializeMap, SerializeSeq },
    Deserialize,
    Serialize,
};

/// Constructed using multiple API calls to get all the types, endpoints, and events.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtendedHelp {
    pub types: Vec<Type>,
    pub endpoints: Vec<Endpoint>,
    pub events: Vec<Event>,
}

/// The base help returned from the LCU API.
#[derive(Serialize, Deserialize, Debug)]
pub struct Help {
    pub events: StringMap,
    pub functions: StringMap,
    pub types: StringMap,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Info {
    pub name: String,
    pub description: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Event {
    #[serde(flatten)]
    pub info: Info,
    #[serde(rename = "nameSpace")]
    pub namespace: String,
    pub tags: Vec<String>,
    #[serde(default, rename = "type")]
    pub ty: DataType,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    #[serde(flatten)]
    pub info: Info,
    #[serde(rename = "nameSpace")]
    pub namespace: String,
    pub help: String,
    pub arguments: Vec<Argument>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<HttpMethod>,
    #[serde(default, deserialize_with = "crate::serde::de::deserialize_option_string")]
    pub path: Option<String>,
    #[serde(default)]
    pub path_params: Vec<String>,
    #[serde(default, rename = "returns")]
    pub return_ty: DataType,
    #[serde(
        rename = "async",
        default,
        deserialize_with = "crate::serde::de::deserialize_bool_coerced"
    )]
    pub is_async: bool,
    #[serde(
        rename = "threadSafe",
        default,
        deserialize_with = "crate::serde::de::deserialize_bool_coerced"
    )]
    pub is_thread_safe: bool,
    #[serde(
        rename = "overridden",
        default,
        deserialize_with = "crate::serde::de::deserialize_bool_coerced"
    )]
    pub is_override: bool,
    #[serde(
        rename = "silentOverride",
        default,
        deserialize_with = "crate::serde::de::deserialize_bool_coerced"
    )]
    pub is_silent_override: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConsoleEndpointInner {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_method: Option<HttpMethod>,
    #[serde(default, deserialize_with = "deserialize_console_url")]
    pub url: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Argument {
    #[serde(flatten)]
    pub info: Info,
    #[serde(
        rename = "optional",
        default,
        deserialize_with = "crate::serde::de::deserialize_bool_coerced"
    )]
    pub is_optional: bool,
    #[serde(default, rename = "type")]
    pub ty: DataType,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Type {
    pub values: Vec<Value>,
    pub fields: Vec<Field>,
    #[serde(flatten)]
    pub info: Info,
    #[serde(rename = "nameSpace")]
    pub namespace: String,
    pub size: usize,
    pub tags: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Value {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub value: serde_json::Value,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    #[serde(flatten)]
    pub info: Info,

    pub offset: usize,

    #[serde(
        rename = "optional",
        default,
        deserialize_with = "crate::serde::de::deserialize_bool_coerced"
    )]
    pub is_optional: bool,

    #[serde(default, rename = "type")]
    pub ty: DataType,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataType {
    pub element_type: String,
    #[serde(default, rename = "type")]
    pub ty: String,
}

impl AsRef<DataType> for DataType {
    fn as_ref(&self) -> &DataType {
        self
    }
}

impl DataType {
    #[inline]
    pub fn string() -> Self {
        Self {
            ty: "string".to_string(),
            element_type: Default::default(),
        }
    }

    /// Returns `true` if the type is `"object"` and the element type is empty.
    #[inline]
    pub fn is_generic_object(&self) -> bool {
        self.ty == "object" && self.element_type.is_empty()
    }
}

/// HTTP verb (only the ones the client exposes)
#[derive(Clone, Copy, Debug, Display, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Trace,
}

impl HttpMethod {
    /// Returns `true` if the http method is [`Get`].
    ///
    /// [`Get`]: HttpMethod::Get
    #[must_use]
    pub fn is_get(&self) -> bool {
        matches!(self, Self::Get)
    }
}

/// Custom visitor:  null / ""  →  GET
impl<'de> Deserialize<'de> for HttpMethod {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = HttpMethod;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("null or an HTTP verb string")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> where E: serde::de::Error {
                Ok(HttpMethod::Get) // null  -> GET
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> where E: serde::de::Error {
                Ok(HttpMethod::Get) // explicit null
            }

            fn visit_some<D>(self, d: D) -> Result<Self::Value, D::Error> where D: Deserializer<'de> {
                Deserialize::deserialize(d).and_then(|s: String| {
                    Ok(match s.to_ascii_uppercase().as_str() {
                        "GET" => HttpMethod::Get,
                        "POST" => HttpMethod::Post,
                        "PUT" => HttpMethod::Put,
                        "PATCH" => HttpMethod::Patch,
                        "DELETE" => HttpMethod::Delete,
                        "HEAD" => HttpMethod::Head,
                        "OPTIONS" => HttpMethod::Options,
                        "TRACE" => HttpMethod::Trace,
                        other => {
                            return Err(
                                serde::de::Error::unknown_variant(
                                    other,
                                    &[
                                        "GET",
                                        "POST",
                                        "PUT",
                                        "PATCH",
                                        "DELETE",
                                        "HEAD",
                                        "OPTIONS",
                                        "TRACE",
                                    ]
                                )
                            );
                        }
                    })
                })
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: serde::de::Error {
                self.visit_some(serde::de::IntoDeserializer::into_deserializer(v))
            }
        }

        deserializer.deserialize_option(Visitor)
    }
}

impl FromStr for HttpMethod {
    type Err = crate::error::ParseError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        if name.starts_with("Get") {
            Ok(Self::Get)
        } else if name.starts_with("Post") {
            Ok(Self::Post)
        } else if name.starts_with("Put") {
            Ok(Self::Put)
        } else if name.starts_with("Patch") {
            Ok(Self::Patch)
        } else if name.starts_with("Delete") {
            Ok(Self::Delete)
        } else if name.starts_with("Head") {
            Ok(Self::Head)
        } else if name.starts_with("Options") {
            Ok(Self::Options)
        } else if name.starts_with("Trace") {
            Ok(Self::Trace)
        } else {
            Err(crate::error::ParseError::UnknownHttpMethod)
        }
    }
}

/// A helper type that will only deserialize the first item of a sequence.
#[derive(Debug, Clone)]
pub struct SeqFirst<T>(pub T);

impl<T> Serialize for SeqFirst<T> where T: Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        let mut seq = serializer.serialize_seq(Some(1))?;
        seq.serialize_element(&self.0)?;
        seq.end()
    }
}

impl<'de, T> Deserialize<'de> for SeqFirst<T> where T: Deserialize<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        struct ExtractFirstListItemVisitor<T>(std::marker::PhantomData<T>);

        impl<'de, T> Visitor<'de> for ExtractFirstListItemVisitor<T> where T: Deserialize<'de> {
            type Value = SeqFirst<T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a list of items")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where A: serde::de::SeqAccess<'de>
            {
                if let Some(value) = seq.next_element()? {
                    Ok(SeqFirst(value))
                } else {
                    Err(serde::de::Error::custom("expected at least one item"))
                }
            }
        }

        deserializer.deserialize_seq(ExtractFirstListItemVisitor(std::marker::PhantomData))
    }
}

#[derive(Debug, Clone, Default)]
pub struct StringMap {
    /// Values that are non-empty strings.
    pub values: FxHashMap<String, String>,
    /// Set of keys that have empty string values.
    pub empty: FxHashSet<String>,
}

impl StringMap {
    /// Get the value for a key, or None if the key is not present or if the value was empty.
    #[allow(dead_code)]
    pub fn get(&self, key: &str) -> Option<&str> {
        if self.empty.contains(key) { None } else { self.values.get(key).map(|s| s.as_str()) }
    }

    /// Returns an iterator over keys in the map.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys().chain(self.empty.iter())
    }

    /// Returns a boolean indicating whether the map contains a key.
    #[allow(dead_code)]
    pub fn contains_key(&self, key: &str) -> bool {
        self.values.contains_key(key) || self.empty.contains(key)
    }

    /// Returns an iterator over entries in the map.
    #[allow(dead_code)]
    pub fn iter(&self) -> impl Iterator<Item = (&str, Option<&str>)> {
        self.values
            .iter()
            .map(|(k, v)| (k.as_str(), Some(v.as_str())))
            .chain(self.empty.iter().map(|k| (k.as_str(), None)))
    }
}

impl Serialize for StringMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        let mut map = serializer.serialize_map(Some(self.values.len() + self.empty.len()))?;
        for (key, value) in &self.values {
            map.serialize_entry(key, value)?;
        }
        for key in &self.empty {
            map.serialize_entry(key, "")?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for StringMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        struct StringMapVisitor;

        impl<'de> Visitor<'de> for StringMapVisitor {
            type Value = StringMap;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map of strings")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where A: serde::de::MapAccess<'de>
            {
                let mut result = StringMap::default();

                while let Some((key, value)) = map.next_entry::<String, String>()? {
                    if value.is_empty() {
                        result.empty.insert(key);
                    } else {
                        result.values.insert(key, value);
                    }
                }

                Ok(result)
            }
        }

        deserializer.deserialize_map(StringMapVisitor)
    }
}

fn deserialize_console_url<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where D: serde::Deserializer<'de>
{
    let s: String = String::deserialize(deserializer)?;
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(if s.starts_with('/') { s } else { format!("/{s}") }))
    }
}
