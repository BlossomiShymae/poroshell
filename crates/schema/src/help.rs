use fxhash::{ FxHashMap, FxHashSet };
use serde::{ de::Visitor, ser::{ SerializeMap, SerializeSeq }, Deserialize, Serialize };

/// Constructed using multiple API calls to get all the types, endpoints, and events.
#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Info {
    pub name: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Event {
    #[serde(flatten)]
    pub info: Info,
    #[serde(rename = "nameSpace")]
    pub namespace: String,
    pub tags: Vec<String>,
    #[serde(rename = "type")]
    pub ty: DataType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    #[serde(flatten)]
    pub info: Info,
    #[serde(rename = "nameSpace")]
    pub namespace: String,
    pub help: String,
    pub arguments: Vec<Argument>,
    pub tags: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_option")]
    pub method: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_option")]
    pub path: Option<String>,
    #[serde(default)]
    pub path_params: Vec<String>,
    #[serde(rename = "returns")]
    pub return_ty: DataType,
    #[serde(rename = "async", default, deserialize_with = "deserialize_bool_any")]
    pub is_async: bool,
    #[serde(rename = "threadSafe", default, deserialize_with = "deserialize_bool_any")]
    pub is_thread_safe: bool,
    #[serde(rename = "overridden", default, deserialize_with = "deserialize_bool_any")]
    pub is_override: bool,
    #[serde(rename = "silentOverride", default, deserialize_with = "deserialize_bool_any")]
    pub is_silent_override: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleEndpointInner {
    #[serde(default, deserialize_with = "deserialize_string_option")]
    pub http_method: Option<String>,
    #[serde(default, deserialize_with = "deserialize_console_url")]
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Argument {
    #[serde(flatten)]
    pub info: Info,
    #[serde(rename = "optional", default, deserialize_with = "deserialize_bool_any")]
    pub is_optional: bool,
    #[serde(rename = "type")]
    pub ty: DataType,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Value {
    pub name: String,
    pub description: String,
    pub value: serde_json::Number,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    #[serde(flatten)]
    pub info: Info,

    pub offset: usize,

    #[serde(rename = "optional", default, deserialize_with = "deserialize_bool_any")]
    pub is_optional: bool,

    #[serde(rename = "type")]
    pub ty: DataType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DataType {
    pub element_type: String,
    #[serde(rename = "type")]
    pub ty: String,
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

/// Coerces a value to a boolean. Handles "broken" JSON values like
/// `true`, `false`, `1`, `0`, `yes`, `no`, etc.
fn deserialize_bool_any<'de, D>(deserializer: D) -> Result<bool, D::Error>
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

/// Converts null, empty strings, and "null" to None, and a non-empty string to Some(string).
/// This is useful for deserializing optional fields that still get sent as empty strings.
fn deserialize_string_option<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where D: serde::Deserializer<'de>
{
    use serde_json::Value;
    use serde::de::{ Error as DError, Unexpected };

    match Value::deserialize(deserializer)? {
        Value::Null => Ok(None),
        Value::String(s) if s.is_empty() => Ok(None),
        Value::String(s) if s == "null" => Ok(None),
        Value::String(s) => Ok(Some(s)),
        v =>
            Err(
                D::Error::invalid_type(
                    Unexpected::Other(&format!("{v:?}")),
                    &"a string, null, or empty string"
                )
            ),
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
