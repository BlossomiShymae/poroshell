//! OpenAPI specification implementation for `lcu_schema`.
//!
//! This module provides a complete Rust representation of the OpenAPI 3.0 specification,
//! allowing for programmatic creation, manipulation, and serialization of OpenAPI documents.
//! The implementation uses serde for serialization/deserialization and includes comprehensive
//! type definitions for all major OpenAPI components.
//!
//! # Features
//!
//! - Strict OpenAPI 3.0 schema Serialization/deserialization via serde
//! - Type-safe representation of OpenAPI components
//! - Support for paths, operations, parameters, schemas, and more
//!
//! # Example
//!
//! ```rust
//! use lcu_schema::openapi::{ OpenApiSpec, OpenApiInfo };
//!
//! let spec = OpenApiSpec::from(OpenApiInfo {
//!     title: "My API".to_string(),
//!     version: "1.0.0".to_string(),
//!     description: Some("A sample API".to_string()),
//! });
//! ```

use std::{ ops::Not, str::FromStr };

use derive_more::{ Deref, DerefMut, From };
use serde::{ ser::SerializeStruct, Deserialize, Deserializer, Serialize };
use fxhash::FxHashMap as HashMap;
use serde_json::Number;

/// The root OpenAPI Specification object.
///
/// This struct represents the entire OpenAPI document and serves as the entry point
/// for constructing and manipulating API specifications.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OpenApiSpec {
    /// The semantic version number of the OpenAPI Specification that this document conforms to.
    pub openapi: String,
    /// Metadata about the API.
    pub info: OpenApiInfo,
    /// The available paths and operations for the API.
    #[serde(
        default,
        serialize_with = "crate::serde::ser::serialize_as_btree_map",
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub paths: HashMap<String, PathItem>,
    /// An element to hold various schemas for the specification.
    #[serde(default, skip_serializing_if = "Components::is_empty")]
    pub components: Components,
    /// A list of tags used by the specification with additional metadata.
    /// Order is dependent on the how the schema is serialized.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,
}

/// The metadata about the API.
///
/// Should be used by tooling as required.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenApiInfo {
    /// The title of the API.
    pub title: String,
    /// The version of the OpenAPI document.
    pub version: String,
    /// A short description of the API.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A tag object used to describe a single tag used by the API.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    /// The name of the tag.
    pub name: String,
    /// A short description for the tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// External documentation for this tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_docs: Option<ExternalDocumentation>,
}

pub use components::{ ArraySchema, RefSchema, * };

/// Module containing component definitions used throughout the OpenAPI specification.
pub mod components {
    use super::*;

    /// Holds a set of reusable objects for different aspects of the OpenAPI Specification.
    ///
    /// Objects defined within component object will have no effect on the API
    /// unless they are explicitly referenced from properties outside the components object (e.g., paths).
    #[derive(Clone, Debug, Default, Deref, DerefMut, PartialEq, Serialize, Deserialize)]
    pub struct Components {
        /// A speed-optimized map of schema objects.
        ///
        /// This map is not cryptographically secure and should not be used for sensitive data.
        #[serde(default, serialize_with = "crate::serde::ser::serialize_as_btree_map")]
        pub schemas: HashMap<String, SchemaObject>,
    }

    /// Represents an OpenAPI schema object which can be either a typed schema or a reference schema.
    ///
    /// This enum allows for both inline schema definitions and references to schema components.
    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, From)]
    pub struct SchemaObject {
        /// A schema with a specific type.
        #[serde(flatten)]
        pub ty: TypedSchema,

        /// Allows retaining arbitrary additional fields like ("minimum", "maximum", etc.)
        #[serde(flatten)]
        pub additional_fields: HashMap<String, serde_json::Value>,
    }

    /// A reference to a schema defined elsewhere.
    ///
    /// This allows for reuse of schema definitions across the API specification.
    #[derive(Clone, Debug, Deref, DerefMut, Serialize, Deserialize, PartialEq, From)]
    pub struct RefSchema {
        /// The reference string, typically in the format "#/components/schemas/{name}".
        #[serde(rename = "$ref")]
        pub ref_: String,
    }

    /// Represents the different types of schemas that can be defined in the OpenAPI specification.
    ///
    /// Each variant corresponds to a different data type with its own schema.
    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase", tag = "type")]
    pub enum TypedSchema {
        /// An array schema with a specific item type.
        Array(ArraySchema),
        /// A string schema, potentially with enumeration values.
        String(StringSchema),
        /// An integer schema with an optional format.
        Integer(IntegerSchema),
        /// A number schema with an optional format.
        Number(NumberSchema),
        /// A boolean schema.
        Boolean,
        /// A reference schema with type tag.
        ///
        /// Unlike the [`SchemaObject::Ref`] variant, this variant is tagged with `type`.
        #[serde(untagged)]
        Ref(RefSchema),
        /// An object schema. This could be a complex object with properties and additional properties.
        #[serde(untagged)]
        Object(ObjectSchema),
    }

    /// Defines an array schema with a specific item type.
    #[derive(Clone, Debug, Deref, DerefMut, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct ArraySchema {
        /// The schema that defines the type of array items.
        pub items: Box<SchemaObject>,
    }

    /// A string schema that can include enumeration values.
    #[derive(Clone, Debug, Default, Deref, DerefMut, PartialEq)]
    pub struct StringSchema(pub Vec<EnumVariant>);

    /// A single variant of an OpenAPI enum.
    ///
    /// Each variant consists of a value (`key`) and an optional display name (`name`).
    /// When serializing, the `name` is preferred if present, otherwise the `key` is used.
    #[derive(Clone, Debug, PartialEq)]
    pub struct EnumVariant {
        /// The display name of the enum variant, if available.
        ///
        /// This is often used to present a human-readable label or identifier.
        pub name: Option<String>,

        /// The raw value of the enum variant.
        ///
        /// This represents the actual OpenAPI enum value and determines behavior when serialized.
        pub key: EnumKey,

        /// The description of the enum variant, if available.
        pub description: Option<String>,
    }

    /// A valid OpenAPI enum value.
    ///
    /// Represents the possible types that can be used as enum values in OpenAPI.
    #[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Hash)]
    #[serde(untagged)]
    pub enum EnumKey {
        /// Represents a `null` enum value.
        #[default]
        None,

        /// A string enum value.
        String(String),

        /// A numeric enum value.
        Number(Number),

        /// A boolean enum value.
        Bool(bool),
    }

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    pub struct IntegerSchema {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub format: Option<IntegerFormat>,
    }

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    pub struct NumberSchema {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub format: Option<NumberFormat>,
    }

    /// Format specifications for integer types.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum IntegerFormat {
        /// Unsigned 8-bit integer
        UInt8,
        /// Unsigned 16-bit integer
        UInt16,
        /// Unsigned 32-bit integer
        UInt32,
        /// Unsigned 64-bit integer
        UInt64,
        /// Signed 8-bit integer
        Int8,
        /// Signed 16-bit integer
        Int16,
        /// Signed 32-bit integer
        Int32,
        /// Signed 64-bit integer
        Int64,
    }

    /// Format specifications for floating-point number types.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum NumberFormat {
        /// Single precision floating point (32-bit)
        Float,
        /// Double precision floating point (64-bit)
        Double,
    }

    /// Represents an object schema in the OpenAPI specification.
    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase", tag = "type", rename = "object")]
    pub struct ObjectSchema {
        /// Map of property names to their schema definitions.
        #[serde(
            default,
            serialize_with = "crate::serde::ser::serialize_as_btree_map",
            skip_serializing_if = "HashMap::is_empty"
        )]
        pub properties: HashMap<String, Box<SchemaObject>>,

        /// Specifies whether and how additional properties are handled.
        #[serde(default)]
        pub additional_properties: AdditionalProperties,

        /// List of required property names.
        #[serde(
            default,
            serialize_with = "crate::serde::ser::serialize_strings_sorted",
            skip_serializing_if = "Vec::is_empty"
        )]
        pub required: Vec<String>,
    }

    /// Represents a validated additional properties schema in an object schema.
    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, From)]
    #[serde(untagged)]
    pub enum AdditionalProperties {
        /// Boolean indicating whether additional properties are allowed.
        /// - `true`: any type of additional properties are allowed.
        /// - `false`: no additional properties are allowed.
        Bool(bool),
        /// Schema to validate additional properties against.
        Schema(Box<SchemaObject>),
    }
}

pub use paths::*;

/// Module containing path-related definitions for the OpenAPI specification.
pub mod paths {
    use super::*;

    /// Describes the operations available on a single path.
    ///
    /// A Path Item may be empty, due to ACL constraints.
    /// The path itself is still exposed to the documentation viewer,
    /// but they will not know which operations and parameters are available.
    #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PathItem {
        /// Reference to another PathItem definition.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ref_: Option<String>,
        /// A short summary for the path.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
        /// A detailed description of the path.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        /// An alternative server array to service all operations in this path.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub servers: Vec<ServerSpec>,
        /// A definition of a GET operation on this path.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub get: Option<Operation>,
        /// A definition of a POST operation on this path.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub post: Option<Operation>,
        /// A definition of a PUT operation on this path.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub put: Option<Operation>,
        /// A definition of a DELETE operation on this path.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub delete: Option<Operation>,
        /// A definition of a OPTIONS operation on this path.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub options: Option<Operation>,
        /// A definition of a HEAD operation on this path.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub head: Option<Operation>,
        /// A definition of a PATCH operation on this path.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub patch: Option<Operation>,
        /// A definition of a TRACE operation on this path.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub trace: Option<Operation>,
    }

    /// Describes a single API operation on a path.
    #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Operation {
        /// The operation's unique identifier.
        /// This is used by tooling to identify the operation.
        /// It can be any string, but it should be unique within the API.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub operation_id: Option<String>,

        /// A short summary of what the operation does.
        #[serde(
            default,
            skip_serializing_if = "crate::serde::ser::option_string_is_none_or_empty",
            deserialize_with = "crate::serde::de::deserialize_option_string"
        )]
        pub summary: Option<String>,

        /// A detailed description of the operation.
        #[serde(
            default,
            skip_serializing_if = "crate::serde::ser::option_string_is_none_or_empty",
            deserialize_with = "crate::serde::de::deserialize_option_string"
        )]
        pub description: Option<String>,

        /// A list of tags for API documentation control.
        #[serde(
            default,
            serialize_with = "crate::serde::ser::serialize_strings_sorted",
            skip_serializing_if = "Vec::is_empty"
        )]
        pub tags: Vec<String>,

        /// A list of parameters for the operation.
        /// This list cannot contain duplicate parameters.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub parameters: Vec<Param>,

        /// The request body for the operation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub request_body: Option<RequestBody>,

        /// A map of possible responses as they are returned from the API.
        /// The key is a string representing the HTTP status code.
        #[serde(
            default,
            serialize_with = "crate::serde::ser::serialize_as_btree_map",
            skip_serializing_if = "HashMap::is_empty"
        )]
        pub responses: HashMap<String, Response>,

        /// A map of possible callbacks as they are returned from the API.
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        pub callbacks: HashMap<String, Callback>,

        /// If true, the operation is deprecated and should not be used.
        #[serde(default, rename = "deprecated", skip_serializing_if = "Not::not")]
        pub is_deprecated: bool,

        /// A map of security requirements for the operation.
        /// This defines the security schemes that are required to execute
        /// the operation.
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        pub security: HashMap<String, Vec<String>>,

        /// An alternative server list to service this operation.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub servers: Vec<ServerSpec>,

        /// Additional external documentation for this operation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub external_docs: Option<ExternalDocumentation>,
    }

    /// Describes a single operation parameter.
    ///
    /// Parameters are defined at the operation level and can be used in the path,
    /// query string, header, or cookie of an API request.
    #[derive(Clone, Debug, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase", tag = "in")]
    pub enum Param {
        /// A parameter that appears in the query string.
        Query {
            /// Common parameter properties independent of location.
            #[serde(flatten)]
            param: ParamSchema,
            /// Determines whether the parameter should allow reserved characters.
            #[serde(default)]
            allow_reserved: bool,
        },
        /// A parameter that appears in the request headers.
        Header(ParamSchema),
        /// A parameter that appears in the path segment of the URL.
        Path(ParamSchema),
        /// A parameter that appears in the cookie.
        Cookie(ParamSchema),
        /// A reference to a parameter defined elsewhere in the OpenAPI document.
        Ref(String),
    }

    /// Common parameter properties independent of location.
    #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ParamSchema {
        /// The name of the parameter.
        pub name: String,

        /// Describes how the parameter value will be serialized.
        pub style: ParamStyle,

        /// Defaultable options for the parameter.
        #[serde(default, flatten)]
        pub options: ParamOptions,
    }

    /// Optional configuration for parameters.
    #[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ParamOptions {
        /// A short description of the parameter.
        #[serde(
            default,
            skip_serializing_if = "crate::serde::ser::option_string_is_none_or_empty",
            deserialize_with = "crate::serde::de::deserialize_option_string"
        )]
        pub description: Option<String>,

        /// The schema defining the type used for the parameter.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub schema: Option<SchemaObject>,

        #[serde(default, rename = "required", skip_serializing_if = "Not::not")]
        pub is_required: bool,

        /// Specifies that the parameter is deprecated and should be
        /// considered obsolete.
        #[serde(default, rename = "deprecated", skip_serializing_if = "Not::not")]
        pub is_deprecated: bool,

        /// Sets the ability to pass empty values.
        #[serde(default, rename = "allowEmptyValue", skip_serializing_if = "Not::not")]
        pub allow_empty_values: bool,

        /// Determines whether the parameter values of arrays and objects
        /// should explode into separate parameters or be serialized as a single
        /// parameter.
        #[serde(default, rename = "explode", skip_serializing_if = "Not::not")]
        pub should_explode: bool,

        /// Example of the parameter's potential value.
        /// This is not a schema, but a sample value.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub example: Option<String>,

        /// Examples of the parameter's potential values.
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        pub examples: HashMap<String, Example>,
    }

    /// Defines how parameter values are serialized depending on the parameter type.
    #[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum ParamStyle {
        /// Matrix style parameters (;param=value;param2=value2)
        Matrix,
        /// Label style parameters (.param.param2)
        Label,
        /// Form style parameters (?param=value&param2=value2)
        Form,
        /// Simple style parameters (comma-separated values)
        Simple,
        /// Space-delimited array values
        SpaceDelimited,
        /// Pipe-delimited array values
        PipeDelimited,
        /// Deep object style parameters (nested objects)
        DeepObject,
    }

    /// Describes a request body for an operation.
    #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
    pub struct RequestBody {
        /// A short description of the request body.
        #[serde(
            default,
            skip_serializing_if = "crate::serde::ser::option_string_is_none_or_empty",
            deserialize_with = "crate::serde::de::deserialize_option_string"
        )]
        pub description: Option<String>,
        /// The content of the request body.
        /// The key is a media type or media type range and the value describes it.
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        pub content: HashMap<String, MediaType>,
        /// Determines if the request body is required in the request.
        #[serde(default, rename = "required")]
        pub is_required: bool,
    }

    /// Describes a single response from an API operation.
    #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
    pub struct Response {
        /// A map containing descriptions of potential response payloads.
        /// The key is a media type or media type range and the value describes it.
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        pub content: HashMap<String, MediaType>,
        #[serde(default, skip_serializing_if = "crate::serde::ser::option_string_is_none_or_empty")]
        /// A description of the response.
        pub description: Option<String>,
    }

    /// Provides schema and examples for a specific media type.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, From)]
    pub struct MediaType {
        /// The schema defining the content of the request, response, or parameter.
        #[serde(default)]
        pub schema: SchemaObject,
    }

    impl Default for SchemaObject {
        fn default() -> Self {
            Self::object_of(true)
        }
    }

    /// A map of out-of-band callbacks related to the parent operation.
    #[derive(Clone, Debug, Default, Deref, DerefMut, PartialEq, Serialize, Deserialize)]
    pub struct Callback(pub HashMap<String, PathItem>);

    /// Example object for OpenAPI parameter examples.
    #[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Example {
        /// A short description of the example.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub summary: Option<String>,
        /// A detailed description of the example.
        #[serde(
            default,
            skip_serializing_if = "crate::serde::ser::option_string_is_none_or_empty",
            deserialize_with = "crate::serde::de::deserialize_option_string"
        )]
        pub description: Option<String>,
        /// Embedded literal example.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<serde_json::Value>, // todo
        /// A URL that points to the literal example.
        #[serde(
            default,
            skip_serializing_if = "crate::serde::ser::option_string_is_none_or_empty",
            deserialize_with = "crate::serde::de::deserialize_option_string"
        )]
        pub external_value: Option<String>,
    }

    /// Additional external documentation.
    #[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ExternalDocumentation {
        /// A description of the target documentation.
        #[serde(
            default,
            skip_serializing_if = "crate::serde::ser::option_string_is_none_or_empty",
            deserialize_with = "crate::serde::de::deserialize_option_string"
        )]
        pub description: Option<String>,
        /// The URL for the target documentation.
        #[serde(
            skip_serializing_if = "crate::serde::ser::option_string_is_none_or_empty",
            deserialize_with = "crate::serde::de::deserialize_option_string"
        )]
        pub url: Option<String>,
    }

    /// Server specification for an OpenAPI operation.
    #[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ServerSpec {
        #[serde(
            default,
            skip_serializing_if = "crate::serde::ser::option_string_is_none_or_empty",
            deserialize_with = "crate::serde::de::deserialize_option_string"
        )]
        pub description: Option<String>,
        /// A URL to the target host.
        /// This URL supports Server Variables and may be relative,
        /// to indicate that the host location is relative to the location of the OAS document.
        /// Variable substitutions will be made when a variable is wrapped in {braces}.
        pub url: String,

        /// A map of variables for server URL template substitution.
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        pub variables: HashMap<String, ServerVariable>,
    }

    /// An object representing a Server Variable for server URL template substitution.
    #[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ServerVariable {
        /// An optional description for the server variable.
        #[serde(
            default,
            skip_serializing_if = "crate::serde::ser::option_string_is_none_or_empty",
            deserialize_with = "crate::serde::de::deserialize_option_string"
        )]
        pub description: Option<String>,
        /// An enumeration of string values to be used if the substitution options are from a limited set.
        ///
        /// The array SHOULD NOT be empty.
        #[serde(rename = "enum", default, skip_serializing_if = "Vec::is_empty")]
        pub enum_values: Vec<String>,
        /// The default value to use for substitution, which MUST be in the enumeration.
        pub default: String,
    }
}

impl From<String> for Tag {
    fn from(name: String) -> Self {
        Self {
            name,
            description: None,
            external_docs: None,
        }
    }
}

impl From<OpenApiInfo> for OpenApiSpec {
    fn from(info: OpenApiInfo) -> Self {
        Self {
            openapi: "3.0.0".to_string(),
            info,
            ..Default::default()
        }
    }
}

impl Components {
    /// Returns `true` if `schemas` is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }
}

macro_rules! try_convert_json {
    ($($ident:ident),* $(,)?) => {
        $(
            impl TryFrom<serde_json::Value> for $ident {
                type Error = crate::error::ParseError;

                fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
                    serde_json::from_value(serde_json::to_value(value)?).map_err(ParseError::from)
                }
            }

            impl TryFrom<$ident> for serde_json::Value {
                type Error = crate::error::ParseError;

                fn try_from(value: $ident) -> Result<Self, Self::Error> {
                    serde_json::from_value(serde_json::to_value(value)?).map_err(ParseError::from)
                }
            }
        )*
    };
}

try_convert_json!(
    ArraySchema,
    Callback,
    Components,
    EnumKey,
    EnumVariant,
    Example,
    ExternalDocumentation,
    IntegerSchema,
    MediaType,
    NumberSchema,
    ObjectSchema,
    OpenApiInfo,
    OpenApiSpec,
    Operation,
    Param,
    ParamOptions,
    ParamSchema,
    PathItem,
    RefSchema,
    RequestBody,
    Response,
    SchemaObject,
    ServerSpec,
    ServerVariable,
    StringSchema,
    Tag
);

impl SchemaObject {
    /// Returns `true` if the schema is [`TypedSchema::Object`].
    ///
    /// [`TypedSchema::Object`]: TypedSchema::Object
    #[must_use]
    pub fn is_object(&self) -> bool {
        matches!(self.ty, TypedSchema::Object(..))
    }

    /// Creates a reference to a component schema.
    #[inline]
    pub fn component_ref(name: &str) -> Self {
        SchemaObject {
            ty: TypedSchema::Ref(RefSchema {
                ref_: format!(
                    "#/components/schemas/{}",
                    name.trim_start_matches("/").trim_start_matches("#/components/schemas/")
                ),
            }),
            additional_fields: Default::default(),
        }
    }

    /// Creates a generic "string" type without any known enum values.
    #[inline]
    pub fn string() -> Self {
        SchemaObject {
            ty: TypedSchema::String(StringSchema::default()),
            additional_fields: Default::default(),
        }
    }

    /// Creates a string schema with the provided enum variants.
    pub fn string_of(variants: impl Into<Vec<EnumVariant>>) -> Self {
        SchemaObject {
            ty: TypedSchema::String(StringSchema(variants.into())),
            additional_fields: Default::default(),
        }
    }

    /// Create a number schema with a specific format.
    #[inline]
    pub fn number(format: impl AsRef<str>) -> Self {
        SchemaObject {
            ty: TypedSchema::Number(NumberSchema {
                format: NumberFormat::from_str(format.as_ref()).ok(),
            }),
            additional_fields: Default::default(),
        }
    }

    /// Create an integer schema with a specific format.
    #[inline]
    pub fn integer(format: impl AsRef<str>) -> Self {
        SchemaObject {
            ty: TypedSchema::Integer(IntegerSchema {
                format: IntegerFormat::from_str(format.as_ref()).ok(),
            }),
            additional_fields: Default::default(),
        }
    }

    /// Creates an empty object schema without any properties or required fields.
    #[inline]
    pub fn object_of(element_ty: impl Into<AdditionalProperties>) -> Self {
        SchemaObject {
            ty: TypedSchema::Object(ObjectSchema {
                properties: Default::default(),
                required: Default::default(),
                additional_properties: element_ty.into(),
            }),
            additional_fields: Default::default(),
        }
    }

    /// Create a boolean schema.
    #[inline]
    pub fn bool() -> Self {
        SchemaObject {
            ty: TypedSchema::Boolean,
            additional_fields: Default::default(),
        }
    }

    pub fn try_parse_item_type(ty: &str) -> Result<Self, crate::error::ParseError> {
        match ty {
            "array" | "vector" => Err(crate::error::ParseError::VectorTypesShouldBeParsed),
            "map" | "object" => Err(crate::error::ParseError::ObjectTypesShouldBeParsed),
            "string" => Ok(SchemaObject::string()),
            "bool" | "boolean" => Ok(SchemaObject::bool()),
            "" => Ok(SchemaObject::object_of(true)),
            other =>
                match other {
                    "float" | "double" => Ok(SchemaObject::number(other)),
                    | "uint8"
                    | "uint16"
                    | "uint32"
                    | "uint64"
                    | "int8"
                    | "int16"
                    | "int32"
                    | "int64" => Ok(SchemaObject::integer(other)),
                    _ => Ok(SchemaObject::component_ref(other)),
                }
        }
    }
}

impl TypedSchema {
    /// Returns `true` if the typed schema is [`Object`].
    ///
    /// [`Object`]: TypedSchema::Object
    #[must_use]
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(..))
    }

    /// Get a reference to the `ObjectSchema` if self is a [`TypedSchema::Object`].
    pub fn as_object(&self) -> Option<&ObjectSchema> {
        if let Self::Object(v) = self { Some(v) } else { None }
    }

    /// Get a reference to the `Array` if self is a [`TypedSchema::Array`].
    pub fn as_string(&self) -> Option<&StringSchema> {
        if let Self::String(v) = self { Some(v) } else { None }
    }

    /// Get a reference to the `Array` if self is a [`TypedSchema::Array`].
    pub fn as_array(&self) -> Option<&ArraySchema> {
        if let Self::Array(v) = self { Some(v) } else { None }
    }

    /// Get a reference to the `Number` if self is a [`TypedSchema::Number`].
    pub fn as_number(&self) -> Option<&NumberSchema> {
        if let Self::Number(v) = self { Some(v) } else { None }
    }

    /// Get a reference to the `Integer` if self is a [`TypedSchema::Integer`].
    pub fn as_integer(&self) -> Option<&IntegerSchema> {
        if let Self::Integer(v) = self { Some(v) } else { None }
    }
}

impl ArraySchema {
    /// Create a new `ArraySchema` of the given type.
    #[inline]
    pub fn of(items: impl Into<Box<SchemaObject>>) -> Self {
        ArraySchema { items: items.into() }
    }
}

impl EnumVariant {
    /// A constant representing a `null` enum variant.
    pub const NONE: Self = EnumVariant { name: None, key: EnumKey::None, description: None };
}

impl EnumKey {
    pub fn string(str: impl Into<String>) -> Self {
        EnumKey::String(str.into())
    }
}

impl FromStr for IntegerFormat {
    type Err = crate::error::ParseError;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        match str {
            "int8" => Ok(IntegerFormat::Int8),
            "int16" => Ok(IntegerFormat::Int16),
            "int32" => Ok(IntegerFormat::Int32),
            "int64" => Ok(IntegerFormat::Int64),
            "uint8" => Ok(IntegerFormat::UInt8),
            "uint16" => Ok(IntegerFormat::UInt16),
            "uint32" => Ok(IntegerFormat::UInt32),
            "uint64" => Ok(IntegerFormat::UInt64),
            _ => Err(crate::error::ParseError::FormatIsNotAnInteger),
        }
    }
}

impl FromStr for NumberFormat {
    type Err = crate::error::ParseError;

    fn from_str(str: &str) -> Result<Self, Self::Err> {
        match str {
            "float" => Ok(NumberFormat::Float),
            "double" => Ok(NumberFormat::Double),
            _ => Err(crate::error::ParseError::FormatIsNotANumber),
        }
    }
}

impl<'de> Deserialize<'de> for StringSchema {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "kebab-case")]
        struct RawStringSchema {
            #[serde(default, rename = "enum", skip_serializing_if = "Vec::is_empty")]
            variants: Vec<EnumVariant>,
            #[serde(default, skip_serializing_if = "Vec::is_empty")]
            x_enum_description: Vec<String>,
        }

        let raw_schema = RawStringSchema::deserialize(deserializer)?;
        let mut variants = raw_schema.variants;

        for (i, description) in raw_schema.x_enum_description.into_iter().enumerate() {
            if i < variants.len() {
                variants[i].description = Some(description);
            }
        }

        Ok(StringSchema(variants))
    }
}

impl Serialize for StringSchema {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        let mut state = serializer.serialize_struct("StringSchema", 2)?;
        if !self.is_empty() {
            state.serialize_field("enum", &self.0)?;
            state.serialize_field("x-enum-description", &self.0)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for EnumVariant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        use serde_json::Value;
        let raw_variant = Value::deserialize(deserializer)?;
        match raw_variant {
            Value::Null => Ok(EnumVariant::NONE),
            Value::Bool(a) =>
                Ok(EnumVariant {
                    name: Some(a.to_string()),
                    key: EnumKey::Bool(a),
                    description: None,
                }),
            Value::Number(a) =>
                Ok(EnumVariant {
                    name: Some(a.to_string()),
                    key: EnumKey::Number(a),
                    description: None,
                }),
            Value::String(a) =>
                Ok(EnumVariant {
                    name: Some(a.clone()),
                    key: EnumKey::String(a),
                    description: None,
                }),
            _ =>
                Err(
                    serde::de::Error::custom(
                        format!("Unsupported enum value type: {:?}", raw_variant)
                    )
                ),
        }
    }
}

impl Serialize for EnumVariant {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        if let Some(name) = &self.name {
            serializer.serialize_str(name)
        } else {
            match &self.key {
                EnumKey::None => serializer.serialize_none(),
                EnumKey::String(s) => serializer.serialize_str(s),
                EnumKey::Number(n) =>
                    serializer.serialize_u64({
                        n
                            .as_u64()
                            .ok_or_else(||
                                serde::ser::Error::custom(format!("Invalid number: {:?}", n))
                            )?
                    }),
                EnumKey::Bool(b) => serializer.serialize_bool(*b),
            }
        }
    }
}

impl PartialOrd for EnumKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use EnumKey::*;
        Some(match (self, other) {
            // Treat none as less than any other value like rust treats Option::None
            (None, None) => std::cmp::Ordering::Equal,
            (None, _) => std::cmp::Ordering::Less,
            (_, None) => std::cmp::Ordering::Greater,

            (Bool(a), Bool(b)) => a.cmp(b),
            (Number(a), Number(b)) => a.to_string().cmp(&b.to_string()), // fallback comparison
            (String(a), String(b)) => a.cmp(b),

            // enforce stable total ordering between types
            (Bool(_), _) => std::cmp::Ordering::Less,
            (Number(_), Bool(_)) => std::cmp::Ordering::Greater,
            (Number(_), _) => std::cmp::Ordering::Less,
            (String(_), _) => std::cmp::Ordering::Greater,
        })
    }
}

/// Sorts a mutable slice of [`EnumVariant`]s.
///
/// - If all variants have a non-`None` key, sorts by key.
/// - Otherwise, sorts by name (with `None` names last).
///
/// # Examples
/// ## Sort by name when some keys are `EnumKey::None`
/// ```
/// use lcu_schema::openapi::{EnumVariant, EnumKey, sort_enum_variants};
/// let mut items = vec![
///     EnumVariant { name: None, key: EnumKey::string("A"), description: None },
///     EnumVariant { name: Some("A".to_string()), key: EnumKey::None, description: None },
///     EnumVariant { name: Some("B".to_string()), key: EnumKey::string("B"), description: None }
/// ];
/// sort_enum_variants(&mut items);
///
/// assert_eq!(items[0].name, None);
/// assert_eq!(items[1].name, Some("A".to_string()));
/// assert_eq!(items[2].name, Some("B".to_string()));
/// ```
///
/// ### Should sort by key when all keys are not [`EnumKey::None`]
/// ```
/// use lcu_schema::openapi::{EnumVariant, EnumKey, sort_enum_variants};
/// let mut items = vec![
///     EnumVariant { name: Some("Y".to_string()), key: EnumKey::string("C"), description: None },
///     EnumVariant { name: None, key: EnumKey::string("B"), description: None },
///     EnumVariant { name: Some("Z".to_string()), key: EnumKey::string("A"), description: None }
/// ];
/// sort_enum_variants(&mut items);
///
/// assert_eq!(items[0].key, EnumKey::string("A"));
/// assert_eq!(items[1].key, EnumKey::string("B"));
/// assert_eq!(items[2].key, EnumKey::string("C"));
/// ```
pub fn sort_enum_variants(variants: &mut [EnumVariant]) {
    if variants.iter().any(|v| matches!(v.key, EnumKey::None)) {
        variants.sort_by(|a, b| a.name.cmp(&b.name));
    } else {
        variants.sort_by(|a, b| a.key.partial_cmp(&b.key).unwrap_or(std::cmp::Ordering::Equal));
    }
}

#[cfg(test)]
mod test_enums {
    use super::*;

    /// Ensure [`EnumKey`] implements [`PartialOrd`] like [`Option`].
    #[test]
    fn enum_key_partial_ord() {
        let a = EnumKey::None;
        let b = EnumKey::string("A");
        let c = EnumKey::string("B");

        assert!(a < b);
        assert!(b < c);
        assert!(a < c);
    }

    /// Ensure [`sort_enum_variants`] sorts by **key** when all keys are not [`EnumKey::None`].
    #[test]
    fn should_sort_by_key_when_all_keys_are_not_none() {
        let mut items = vec![
            EnumVariant {
                name: Some("Y".to_string()),
                key: EnumKey::string("C"),
                description: None,
            },
            EnumVariant { name: None, key: EnumKey::string("B"), description: None },
            EnumVariant {
                name: Some("Z".to_string()),
                key: EnumKey::string("A"),
                description: None,
            }
        ];
        sort_enum_variants(&mut items);

        assert_eq!(items[0].key, EnumKey::string("A"));
        assert_eq!(items[1].key, EnumKey::string("B"));
        assert_eq!(items[2].key, EnumKey::string("C"));
    }

    /// Ensure [`sort_enum_variants`] sorts by **name** when some keys are [`EnumKey::None`].
    #[test]
    fn should_sort_by_name_when_some_keys_are_none() {
        let mut items = vec![
            EnumVariant { name: None, key: EnumKey::string("A"), description: None },
            EnumVariant { name: Some("A".to_string()), key: EnumKey::None, description: None },
            EnumVariant {
                name: Some("B".to_string()),
                key: EnumKey::string("B"),
                description: None,
            }
        ];
        sort_enum_variants(&mut items);

        assert_eq!(items[0].name, None);
        assert_eq!(items[1].name, Some("A".to_string()));
        assert_eq!(items[2].name, Some("B".to_string()));
    }

    /// Test to ensure that the enum variants are preserved in the order they are deserialized
    /// and that the descriptions are correctly associated with the variants.
    #[test]
    fn should_fold_enum_and_description_and_preserve_order() {
        use serde_json::json;

        let ty_schema =
            json!({
            "type": "string",
            "enum": ["B", "A", "C"],
            "x-enum-description": ["B-Desc", "A-Desc", "C-Desc"],
        });
        let ty_schema: TypedSchema = serde_json::from_value(ty_schema).unwrap();
        println!("ty_schema: {:#?}", ty_schema);
        let expected = TypedSchema::String(
            StringSchema(
                vec![
                    EnumVariant {
                        name: Some("B".to_string()),
                        key: EnumKey::String("B".to_string()),
                        description: Some("B-Desc".to_string()),
                    },
                    EnumVariant {
                        name: Some("A".to_string()),
                        key: EnumKey::String("A".to_string()),
                        description: Some("A-Desc".to_string()),
                    },
                    EnumVariant {
                        name: Some("C".to_string()),
                        key: EnumKey::String("C".to_string()),
                        description: Some("C-Desc".to_string()),
                    }
                ]
            )
        );
        assert_eq!(ty_schema, expected);
    }
}

impl<T> From<T> for AdditionalProperties where T: Into<SchemaObject> {
    fn from(schema_obj: T) -> Self {
        AdditionalProperties::Schema(Box::new(schema_obj.into()))
    }
}

impl Default for AdditionalProperties {
    fn default() -> Self {
        AdditionalProperties::Bool(false)
    }
}

impl ParamStyle {
    /// The default style for query parameters.
    #[inline]
    pub const fn default_query() -> Self {
        ParamStyle::Form
    }

    /// The default style for header parameters.
    #[inline]
    pub const fn default_header() -> Self {
        ParamStyle::Simple
    }

    /// The default style for path parameters.
    #[inline]
    pub const fn default_path() -> Self {
        ParamStyle::Simple
    }

    /// The default style for cookie parameters.
    #[inline]
    pub const fn default_cookie() -> Self {
        ParamStyle::Form
    }
}

impl<'de> Deserialize<'de> for Param {
    /// Special deserialization for PathParam that converts `in` field to an enum container and
    /// resolves default `style` when not provided.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawParam {
            name: String,
            #[serde(flatten)]
            options: ParamOptions,
            style: Option<ParamStyle>,
            #[serde(rename = "in")]
            in_: String,
            #[serde(default)]
            allow_reserved: bool,
        }

        let RawParam { name, options, style, in_, allow_reserved } =
            RawParam::deserialize(deserializer)?;
        let in_ = in_.to_lowercase();

        match in_.as_str() {
            "query" => {
                let style = style.unwrap_or_else(ParamStyle::default_query);
                Ok(Param::Query { param: ParamSchema { name, style, options }, allow_reserved })
            }
            "header" => {
                let style = style.unwrap_or_else(ParamStyle::default_header);
                Ok(Param::Header(ParamSchema { name, style, options }))
            }
            "path" => {
                let style = style.unwrap_or_else(ParamStyle::default_path);
                Ok(Param::Path(ParamSchema { name, style, options }))
            }
            "cookie" => {
                let style = style.unwrap_or_else(ParamStyle::default_cookie);
                Ok(Param::Cookie(ParamSchema { name, style, options }))
            }
            other =>
                Err(
                    serde::de::Error::unknown_variant(other, &["query", "header", "path", "cookie"])
                ),
        }
    }
}

pub trait Descriptable where Self: Sized {
    /// Returns a mutable reference to the description field.
    fn description_mut(&mut self) -> &mut Option<String>;

    /// Consumes the current instance and returns a new instance with the prodivded description.
    fn with_description(mut self, description: Option<impl Into<String>>) -> Self {
        *self.description_mut() = description.map(Into::into);
        self
    }
}

macro_rules! describe {
    ($($name:ident),*) => {
        $(
            impl Descriptable for $name {
                fn description_mut(&mut self) -> &mut Option<String> {
                    &mut self.description
                }
            }
        )*
    };
}

describe!(
    OpenApiInfo,
    Tag,
    EnumVariant,
    PathItem,
    Operation,
    ParamOptions,
    Response,
    RequestBody,
    Example,
    ExternalDocumentation,
    ServerSpec,
    ServerVariable
);

impl Descriptable for ParamSchema {
    fn description_mut(&mut self) -> &mut Option<String> {
        &mut self.options.description
    }
}

/// A trait for types that can have body content added to them.
pub trait InsertBodyContent where Self: Sized {
    /// Returns a mutable reference to the content map.
    fn content_mut(&mut self) -> &mut HashMap<String, MediaType>;

    /// Consume the current instance and return a new instance with the specified content added.
    fn with_content(mut self, key: impl Into<String>, media_type: impl Into<MediaType>) -> Self {
        self.content_mut().insert(key.into(), media_type.into());
        self
    }
}

impl InsertBodyContent for RequestBody {
    fn content_mut(&mut self) -> &mut HashMap<String, MediaType> {
        &mut self.content
    }
}

impl InsertBodyContent for Response {
    fn content_mut(&mut self) -> &mut HashMap<String, MediaType> {
        &mut self.content
    }
}

use crate::{ error::ParseError, help };

impl TryFrom<&help::Type> for SchemaObject {
    type Error = crate::error::ParseError;

    fn try_from(ty: &help::Type) -> Result<Self, Self::Error> {
        let is_object = !ty.fields.is_empty();
        let is_enum = !ty.values.is_empty();
        assert!(!(is_object && is_enum), "Type cannot be both an object and enum");

        let mut required = Vec::<String>::new();
        let mut properties = HashMap::<String, Box<SchemaObject>>::default();

        for field in &ty.fields {
            if properties.contains_key(&field.info.name) {
                println!("Skipping duplicate field name: {} in {}", field.info.name, ty.info.name);
            } else {
                match SchemaObject::try_from(&field.ty) {
                    Ok(schema) => {
                        properties.insert(field.info.name.clone(), schema.into());
                        if !field.is_optional {
                            required.push(field.info.name.clone());
                        }
                    }
                    Err(Self::Error::PrivateApiTypeNotSupported) => {
                        println!("Skipping field with unsupported type: {}", field.info.name);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }

        if is_object {
            Ok(SchemaObject {
                ty: TypedSchema::Object(ObjectSchema {
                    properties,
                    required,
                    additional_properties: AdditionalProperties::default(),
                }),
                additional_fields: Default::default(),
            })
        } else if is_enum {
            ty.values
                .iter()
                .map(|v| {
                    use serde_json::Value::*;
                    Ok(EnumVariant {
                        key: match &v.value {
                            Null => EnumKey::None,
                            Bool(a) => EnumKey::Bool(*a),
                            Number(a) => EnumKey::Number(a.clone()),
                            String(a) => EnumKey::String(a.clone()),
                            _ => {
                                return Err(
                                    crate::error::ParseError::Json(
                                        serde::ser::Error::custom(
                                            format!("Unsupported enum value type: {:?}", v.value)
                                        )
                                    )
                                );
                            }
                        },
                        name: v.name.is_empty().not().then_some(v.name.clone()),
                        description: v.description
                            .is_empty()
                            .not()
                            .then_some(v.description.clone()),
                    })
                })
                .collect::<Result<Vec<_>, ParseError>>()
                .map(|mut variants| {
                    sort_enum_variants(&mut variants);
                    SchemaObject::string_of(variants)
                })
        } else {
            Err(crate::error::ParseError::PrivateApiTypeNotSupported)
        }
    }
}

impl TryFrom<&help::DataType> for SchemaObject {
    type Error = crate::error::ParseError;

    fn try_from(data_type: &help::DataType) -> Result<Self, Self::Error> {
        if data_type.is_generic_object() {
            return Ok(SchemaObject::object_of(true));
        }

        match SchemaObject::try_parse_item_type(&data_type.ty) {
            Err(Self::Error::VectorTypesShouldBeParsed) =>
                Ok(SchemaObject {
                    ty: TypedSchema::Array(ArraySchema {
                        items: {
                            match SchemaObject::try_parse_item_type(&data_type.element_type) {
                                Ok(schema) => Box::new(schema),
                                Err(Self::Error::ObjectTypesShouldBeParsed) =>
                                    Box::new(SchemaObject::object_of(true)),
                                Err(e) => {
                                    return Err(e);
                                }
                            }
                        },
                    }),
                    additional_fields: Default::default(),
                }),
            Err(Self::Error::ObjectTypesShouldBeParsed) =>
                Ok(SchemaObject {
                    ty: TypedSchema::Object(ObjectSchema {
                        properties: Default::default(),
                        required: Default::default(),
                        additional_properties: {
                            match SchemaObject::try_parse_item_type(&data_type.element_type) {
                                Ok(schema) => AdditionalProperties::from(schema),
                                Err(Self::Error::ObjectTypesShouldBeParsed) =>
                                    AdditionalProperties::from(SchemaObject::object_of(true)),
                                Err(e) => {
                                    return Err(e);
                                }
                            }
                        },
                    }),
                    additional_fields: Default::default(),
                }),
            res => res,
        }
    }
}

pub trait Normalize {
    /// Normalize self.
    fn normalize(self) -> Self where Self: Sized {
        let mut this = self;
        this.normalize_mut();
        this
    }

    /// Mutably normalize self.
    fn normalize_mut(&mut self);
}

impl Normalize for OpenApiSpec {
    fn normalize_mut(&mut self) {
        for (_, schema) in self.components.schemas.iter_mut() {
            schema.normalize_mut();
        }
    }
}

impl Normalize for SchemaObject {
    /// Mutably
    /// - convert all untyped refs to typed refs.
    /// - sort enum values using [`sort_enum_variants`].
    ///
    /// This is useful when comparing alternative schema solutions with `lcu_schema`
    /// and you want to do a basic normalization pass on the input data to align it
    /// more closely to `lcu_schema`'s strict schema validation before diffing the two.
    ///
    /// This is not a full auto-correction pass, but it does help with some common cases.
    fn normalize_mut(&mut self) {
        match &mut self.ty {
            TypedSchema::Object(ObjectSchema { properties, required, additional_properties }) => {
                for (_, schema) in properties.iter_mut() {
                    schema.normalize_mut();
                }
                // Sort the required fields.
                required.sort();
                // Sort the enum values.
                if let AdditionalProperties::Schema(schema) = additional_properties {
                    schema.normalize_mut();
                }
            }
            TypedSchema::Array(ArraySchema { items }) => {
                items.normalize_mut();
            }
            TypedSchema::String(StringSchema(variants)) => {
                sort_enum_variants(variants);
            }
            _ => {}
        }
    }
}
