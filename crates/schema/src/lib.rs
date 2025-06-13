use std::{ collections::HashSet, pin::Pin, str::FromStr };

use fxhash::FxHashMap as HashMap;
#[cfg(feature = "irelia")]
use irelia::{
    error::Error as IreliaError,
    requests::{ HyperError, RequestClientTrait },
    rest::LcuClient,
};
use itertools::Itertools;

use error::{ ParseError, PoroError };
use openapi::{ components::{ RefSchema, * }, paths::*, * };
use patch::Patch;
use help::{
    ConsoleEndpointInner,
    DataType,
    Endpoint,
    Event,
    ExtendedHelp,
    Help,
    HttpMethod,
    SeqFirst,
    Type,
};
use ::serde::{ de::DeserializeOwned, Deserialize, Serialize };

/// `use lcu_schema::prelude::*;` to import common traits and types.
pub mod prelude {
    #[cfg(feature = "irelia")]
    pub use super::lcu;
    pub use super::{
        PoroClient,
        PoroSchema,
        help::ExtendedHelp,
        openapi::OpenApiSpec,
        error::{ ParseError, PoroError },
    };
}

pub mod serde;
pub mod help;
pub mod error;
pub mod openapi;
pub mod patch;

/// Create a new irelia client.
#[cfg(feature = "irelia")]
#[inline]
pub fn lcu() -> Result<impl PoroSchema, PoroError<IreliaError<HyperError>>> {
    let req = irelia::requests::new();
    let lcu = LcuClient::connect_with_request_client(&req).map_err(PoroError::Client)?;
    Ok(lcu)
}

/// A trait for generating LCU API specifications.
pub trait PoroSchema {
    type Error: std::error::Error + Send + Sync;

    /// Construct [`ExtendedHelp`] using the LCU API.
    ///
    /// This will take a while to finish because it queries the LCU API for all
    /// types, events, and endpoints.
    fn extended_help(
        &mut self
    ) -> impl std::future::Future<Output = Result<ExtendedHelp, PoroError<Self::Error>>> + Send;

    /// Construct [`OpenApiSpec`] using the LCU API.
    ///
    /// This is faster than [`extended_help`] because it only queries the LCU API for
    /// the endpoints, but it requires an [`ExtendedHelp`] to be constructed first.
    ///
    /// This still uses the poro client to get the current version of the API, so ideally the [`ExtendedHelp`]
    /// is freshly constructed and up-to-date when used outside of test cases.
    fn openapi(
        &mut self,
        extended_help: ExtendedHelp
    ) -> impl std::future::Future<Output = Result<OpenApiSpec, PoroError<Self::Error>>> + Send;
}

pub trait PoroClient {
    type Error: std::error::Error + Send + Sync;

    /// Make a GET request to the LCU API.
    fn get_lcu<D: DeserializeOwned + Send>(
        &mut self,
        endpoint: impl AsRef<str> + Send
    ) -> Pin<Box<dyn Future<Output = Result<D, Self::Error>> + Send + '_>>;

    /// Make a POST request to the LCU API.
    fn post_lcu<'a, D: DeserializeOwned + Send>(
        &'a mut self,
        endpoint: impl AsRef<str> + Send,
        body: impl Serialize + Send + 'a
    ) -> Pin<Box<dyn Future<Output = Result<D, Self::Error>> + Send + 'a>>;
}

#[cfg(feature = "irelia")]
impl<T: RequestClientTrait + Clone + Send> PoroClient
    for LcuClient<T>
    where <T as RequestClientTrait>::Error: Sync
{
    type Error = IreliaError<<T as RequestClientTrait>::Error>;

    fn get_lcu<D: DeserializeOwned + Send>(
        &mut self,
        endpoint: impl AsRef<str> + Send
    ) -> Pin<Box<dyn Future<Output = Result<D, Self::Error>> + Send + '_>> {
        let endpoint = endpoint.as_ref().to_string();
        let fut = async move {
            let result: D = self.get(&endpoint).await?;
            Ok(result)
        };
        Box::pin(fut)
    }

    fn post_lcu<'a, D: DeserializeOwned + Send>(
        &'a mut self,
        endpoint: impl AsRef<str> + Send,
        body: impl Serialize + Send + 'a
    ) -> Pin<Box<dyn Future<Output = Result<D, Self::Error>> + Send + 'a>> {
        let endpoint = endpoint.as_ref().to_string();
        let fut = async move {
            let result: D = self.post(&endpoint, body).await?;
            Ok(result)
        };
        Box::pin(fut)
    }
}

macro_rules! apply_endpoint_patches {
    (to: $jsons:expr, patches: [$(($name:expr, $(($path:expr, $value:expr)),* $(,)?)),* $(,)?]) => {
        {
            for json in $jsons.iter_mut() {
                
                $(
                    if json.get("name").and_then(|v| v.as_str()).unwrap() == $name {
                        $(
                            json.patch_mut($path, Some($value.into()))?;
                        )*
                    }
                )*
            }
        }
    };
}

#[cfg(test)]
mod test_macros {
    use super::*;

    #[test]
    fn test_apply_endpoint_patches_macro() -> Result<(), ParseError> {
        let mut jsons = vec![
            serde_json::json!({ "name": "Help", "method": "get", "path": "/help" }),
            serde_json::json!({ "name": "Subscribe", "method": "get", "path": "/subscribe" })
        ];

        apply_endpoint_patches!(
            to: jsons,
            patches: [
                ("Help", ("method", "post"), ("path", "/Help")),
                ("Subscribe", ("method", "post"), ("path", "/Subscribe")),
            ]
        );

        assert_eq!(jsons[0]["method"], "post");
        assert_eq!(jsons[0]["path"], "/Help");
        assert_eq!(jsons[1]["method"], "post");
        assert_eq!(jsons[1]["path"], "/Subscribe");

        Ok(())
    }
}

impl<T: PoroClient + Send> PoroSchema for T {
    type Error = <T as PoroClient>::Error;

    async fn extended_help(&mut self) -> Result<ExtendedHelp, PoroError<Self::Error>> {
        let help: Help = self.post_lcu("/help", "").await.map_err(PoroError::Client)?;

        // construct the extended help object
        let mut full_types = Vec::<Type>::new();
        let mut full_events = Vec::<Event>::new();
        let mut full_endpoints = Vec::<serde_json::Value>::new();

        // Get help for all types
        for ty_name in help.types.keys() {
            let endpoint = format!("/help?target={ty_name}&format=Full");
            let SeqFirst::<Type>(full) = self
                .post_lcu(endpoint, "").await
                .map_err(PoroError::Client)?;
            full_types.push(full);
        }

        // Get help for all events
        for ev_name in help.events.keys() {
            let endpoint = format!("/help?target={ev_name}&format=Full");
            let SeqFirst::<Event>(full) = self
                .post_lcu(endpoint, "").await
                .map_err(PoroError::Client)?;
            full_events.push(full);
        }

        // Get help for all endpoints
        let reg = regex::Regex::new(r"\{(.*?)\}");
        for fn_name in help.functions.keys() {
            let endpoint = format!("/help?target={fn_name}&format=Full");
            let SeqFirst::<Endpoint>(mut endpoint) = self
                .post_lcu(endpoint, "").await
                .map_err(PoroError::Client)?;

            if
                // endpoint.info.name == "GetLolRankedV1GlobalNotifications" ||
                endpoint.info.name == "GetLolRankedV1GlobalNotifications" ||
                endpoint.info.name == "PostPlayerNotificationsV1Notifications"
            {
                println!("Endpoint {} is:\n{:#?}", endpoint.info.name, endpoint);
            }

            // Finish construction using data from console help.
            {
                let console = format!("/help?target={fn_name}&format=Console");
                let mut console: serde_json::Value = self
                    .post_lcu(console, "").await
                    .map_err(PoroError::Client)?;
                let console = console
                    .as_object_mut()
                    .ok_or(ParseError::ConsoleEndpointResponseShouldBeObject)?;

                if let Some(console) = console.remove(fn_name) {
                    let console: ConsoleEndpointInner = serde_json::from_value(console)?;

                    endpoint.method = Some(
                        if let Some(method) = console.http_method {
                            method
                        } else {
                            HttpMethod::from_str(&endpoint.info.name).unwrap_or(HttpMethod::Get)
                        }
                    );

                    endpoint.path_params = if let Some(url) = console.url.as_ref() {
                        reg.clone()
                            .unwrap()
                            .captures_iter(url.as_str())
                            .map(|cap| cap[1].to_string())
                            .collect::<Vec<_>>()
                    } else {
                        Vec::new()
                    };
                    endpoint.path = console.url;
                }
            }
            let endpoint = serde_json::to_value(endpoint)?;
            full_endpoints.push(endpoint);
        }

        // Apply endpoint patches
        apply_endpoint_patches!(
            to: &mut full_endpoints,
            patches: [
                ("Help", 
                    ("method", "post"), 
                    ("path", "/Help")
                ),
                ("Subscribe", 
                    ("method", "post"), 
                    ("path", "/Subscribe")
                ),
                ("Unsubscribe", 
                    ("method", "post"), 
                    ("path", "/Unsubscribe")
                ),
                ("AsyncDelete", 
                    ("method", "post"), 
                    ("path", "/AsyncDelete")
                ),
                ("AsyncResult", 
                    ("method", "post"), 
                    ("path", "/AsyncResult")
                ),
                ("AsyncStatus", 
                    ("method", "post"), 
                    ("path", "/AsyncStatus")
                ),
                ("Cancel", 
                    ("method", "post"), 
                    ("path", "/Cancel")
                ),
                ("Exit", 
                    ("method", "post"), 
                    ("path", "/Exit")
                ),
                ("WebSocketFormat", 
                    ("method", "post"), 
                    ("path", "/WebSocketFormat")
                ),
                ("LoggingGetEntries", 
                    ("method", "post"), 
                    ("path", "/LoggingGetEntries")
                ),
                ("LoggingMetrics", 
                    ("method", "post"), 
                    ("path", "/LoggingMetrics")
                ),
                ("LoggingMetricsMetadata", 
                    ("method", "post"), 
                    ("path", "/LoggingMetricsMetadata")
                ),
                ("LoggingStart", 
                    ("method", "post"), 
                    ("path", "/LoggingStart")
                ),
                ("LoggingStop", 
                    ("method", "post"), 
                    ("path", "/LoggingStop")
                ),
                ("GetRiotclientRegionLocale", 
                    ("tags", vec!["riotclient"])
                )
            ]
        );

        let full_endpoints = full_endpoints
            .into_iter()
            .map(|json| {
                match serde_json::from_value::<Endpoint>(json.clone()) {
                    Ok(endpoint) => {
                        if endpoint.info.name == "GetLolRankedV1GlobalNotifications" {
                            println!("Endpoint JSON {} is:\n{:#?}", endpoint.info.name, json);
                        }
                        endpoint
                    }
                    Err(e) => {
                        panic!("Something major changed in the API. Please report this error: {e}");
                    }
                }
            })
            .collect::<Vec<Endpoint>>();

        println!("Total Types: {}", full_types.len());
        println!("Total Endpoints: {}", full_endpoints.len());
        println!("Total Events: {}", full_events.len());

        Ok(ExtendedHelp {
            types: full_types,
            endpoints: full_endpoints,
            events: full_events,
        })
    }

    async fn openapi(
        &mut self,
        extended_help: ExtendedHelp
    ) -> Result<OpenApiSpec, PoroError<Self::Error>> {
        let info = {
            #[derive(Deserialize)]
            struct Version {
                version: String,
            }
            let Version { version } = self
                .get_lcu("/system/v1/builds").await
                .map_err(PoroError::Client)?;

            OpenApiInfo {
                title: "LCU PORO-SCHEMA".to_string(),
                description: Some("OpenAPI v3 specification for LCU".to_string()),
                version,
            }
        };

        OpenApiSpec::from(info)
            .with_components(&extended_help)?
            .with_paths(&extended_help)?
            .with_tags()
            .map(Ok)
    }
}

impl OpenApiSpec {
    /// Map self to a new type using the provided function.
    #[inline]
    pub fn map<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }

    /// Consume the spec and return a new spec with resolved components.
    fn with_components(mut self, help: &ExtendedHelp) -> Result<Self, ParseError> {
        self.resolve_components(help)?;
        Ok(self)
    }

    /// Consume the spec and return a new spec with resolved paths.
    fn with_paths(mut self, help: &ExtendedHelp) -> Result<Self, ParseError> {
        self.resolve_paths(help)?;
        Ok(self)
    }

    /// Consume the spec and return a new spec with resolved tags.
    fn with_tags(mut self) -> Self {
        self.resolve_tags();
        self
    }

    /// Mutably resolve components from the extended help.
    ///
    /// This method iterates over all types in the `extended_help` and attempts to convert them into
    /// [`SchemaObject`]s, which are then inserted into the `components` map of the OpenAPI spec.
    fn resolve_components(&mut self, extended_help: &ExtendedHelp) -> Result<(), ParseError> {
        for ty in &extended_help.types {
            match SchemaObject::try_from(ty) {
                Ok(schema) => {
                    self.components.insert(ty.info.name.clone(), schema);
                }
                Err(ParseError::PrivateApiTypeNotSupported) => {
                    // Silentily ignore this error
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// Mutably resolve paths from the extended help.
    ///
    /// Create [`PathItem`]s for each endpoint in `extended_help` and populate them with [`Operation`]s.
    fn resolve_paths(&mut self, extended_help: &ExtendedHelp) -> Result<(), ParseError> {
        for endpoint in &extended_help.endpoints {
            let Some(path) = &endpoint.path else {
                println!("Endpoint {} does not have an path", endpoint.info.name);
                continue;
            };

            let operation = endpoint.operation(&*self)?;
            let entry: &mut _ = self.paths.entry(path.to_string()).or_default();

            use help::HttpMethod::*;
            match endpoint.method {
                Some(method) => {
                    match method {
                        Get => {
                            entry.get = Some(operation);
                        }
                        Post => {
                            entry.post = Some(operation);
                        }
                        Put => {
                            entry.put = Some(operation);
                        }
                        Delete => {
                            entry.delete = Some(operation);
                        }
                        Patch => {
                            entry.patch = Some(operation);
                        }
                        Options => {
                            entry.options = Some(operation);
                        }
                        Head => {
                            entry.head = Some(operation);
                        }
                        Trace => {
                            entry.trace = Some(operation);
                        }
                    }
                }
                _ => {
                    continue;
                }
            }
        }
        Ok(())
    }

    /// Mutably resolve tags for the OpenAPI spec.
    ///
    /// This method processes the tags from all operations in the spec, filtering and normalizing them,
    /// and then finally collecting them into the root `tags` field of the spec.
    fn resolve_tags(&mut self) {
        // First pass: collect all tags and count them
        let mut tag_counts: HashMap<String, usize> = HashMap::default();
        for path_item in self.paths.values() {
            for operation in path_item.operations() {
                for tag in &operation.tags {
                    *tag_counts.entry(tag.clone()).or_insert(0) += 1;
                }
            }
        }

        // Second pass: filter, assign defaults, and normalize
        let mut final_tags = HashSet::new();
        for (path, path_item) in self.paths.iter_mut() {
            for operation in path_item.operations_mut() {
                // Filter tags based on count or "Plugin" prefix
                let mut filtered_tags: Vec<String> = operation.tags
                    .iter()
                    .filter(|inner| {
                        tag_counts.get(*inner).unwrap_or(&0) > &1 || inner.starts_with("Plugin")
                    })
                    .cloned()
                    .collect();

                // Assign default tag if no tags remain
                if filtered_tags.is_empty() {
                    let tag = path.split('/').nth(1).unwrap_or("other");
                    filtered_tags.push(tag.to_string());
                }

                // Normalize tags to lowercase non-Plugin tags, remove '$'.
                filtered_tags = filtered_tags
                    .into_iter()
                    .map(|tag| {
                        let tag = if tag.starts_with("Plugin ") { tag } else { tag.to_lowercase() };
                        tag.replacen("$", "", 1)
                    })
                    .collect();

                // Extend to final set and update operation
                final_tags.extend(filtered_tags.iter().cloned());
                operation.tags = filtered_tags;
            }
        }

        // Sort tags
        let tags: Vec<Tag> = final_tags
            .into_iter()
            .map(Tag::from)
            .sorted_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .collect();

        // Assign sorted tags to spec
        self.tags = tags;
    }
}

impl PartialOrd for Tag {
    fn partial_cmp(&self, b: &Self) -> Option<std::cmp::Ordering> {
        let a_is_plugin = self.name.starts_with("Plugin");
        let b_is_plugin = b.name.starts_with("Plugin");
        let a_is_lol_plugin = self.name.starts_with("Plugin lol");
        let b_is_lol_plugin = b.name.starts_with("Plugin lol");
        let a_other = self.name == "other";
        let b_other = b.name == "other";

        // Plugin vs non-Plugin
        match (a_is_plugin, b_is_plugin) {
            (true, false) => {
                return Some(std::cmp::Ordering::Greater);
            }
            (false, true) => {
                return Some(std::cmp::Ordering::Less);
            }
            _ => {}
        }

        // Plugin lol vs non-lol Plugin
        match (a_is_lol_plugin, b_is_lol_plugin) {
            (true, false) => {
                return Some(std::cmp::Ordering::Greater);
            }
            (false, true) => {
                return Some(std::cmp::Ordering::Less);
            }
            _ => {}
        }

        // "other" always comes last
        match (a_other, b_other) {
            (true, false) => {
                return Some(std::cmp::Ordering::Greater);
            }
            (false, true) => {
                return Some(std::cmp::Ordering::Less);
            }
            (true, true) => {
                return None;
            }
            _ => {}
        }

        // Otherwise, sort alphabetically
        Some(self.name.cmp(&b.name))
    }
}

impl Endpoint {
    /// Returns a reusable closure that resolves a **path** parameter for an endpoint operation.
    ///
    /// This is used to create [`PathParam::Path`] parameters from [`Endpoint::path_params`].
    #[inline]
    fn path_param_as_path_variant(&self) -> impl (Fn(&String) -> Result<Param, ParseError>) + '_ {
        |param: &String| -> Result<Param, ParseError> {
            let schema = self.arguments
                .iter()
                .find(|arg| { arg.info.name.replacen("+", "", 1) == param.replacen("+", "", 1) })
                .map(|a| { SchemaObject::try_from(&a.ty) })
                .unwrap_or_else(|| { SchemaObject::try_from(&DataType::string()) })?;
            Ok(
                Param::Path(ParamSchema {
                    name: param.clone(),
                    style: ParamStyle::default_path(),
                    options: ParamOptions {
                        schema: Some(schema),
                        is_required: true,
                        ..Default::default()
                    },
                })
            )
        }
    }

    /// Create an [`Operation`] from the endpoint (`&self`). The spec (`spec`)
    /// is used to resolve the schemas for the parameters and request body.
    pub fn operation(&self, spec: &OpenApiSpec) -> Result<Operation, ParseError> {
        let mut request_body = None;
        let mut params: Vec<Param> = self.path_params
            .iter()
            .map(self.path_param_as_path_variant())
            .collect::<Result<Vec<_>, ParseError>>()?;

        let non_path_args_iter = self.arguments.iter().skip(self.path_params.len());
        // If there is only 1 argument, it is the body. So we check > 1.
        if non_path_args_iter.clone().count() > 1 {
            for arg in non_path_args_iter {
                params.push(Param::Query {
                    param: ParamSchema {
                        name: arg.info.name.clone(),
                        style: ParamStyle::default_query(),
                        options: ParamOptions {
                            schema: Some(SchemaObject::try_from(&arg.ty)?),
                            is_required: !arg.is_optional,
                            ..Default::default()
                        },
                    },
                    allow_reserved: false,
                });
            }
        } else {
            /// If there is only 1 argument, it is the body.
            /// We need to determine if it is a body or a query parameter.
            use help::HttpMethod::*;
            match self.method {
                Some(Get) | Some(Delete) | Some(Head) | Some(Options) | Some(Trace) => {
                    for arg in self.arguments.iter().skip(params.len()) {
                        let schema: SchemaObject = (&arg.ty).try_into()?;
                        let schema = match &schema.ty {
                            TypedSchema::Ref(RefSchema { ref_ }) => {
                                let Some(schema) = spec.components.get(
                                    ref_.split("/").last().unwrap()
                                ) else {
                                    eprintln!("Schema {} not found in spec components", ref_);
                                    return Err(ParseError::InvalidData);
                                };
                                schema
                            }
                            _ => &schema,
                        };

                        match &schema.ty {
                            TypedSchema::Object(obj) => {
                                for (name, ty) in &obj.properties {
                                    params.push(Param::Query {
                                        param: ParamSchema {
                                            name: name.clone(),
                                            style: ParamStyle::default_query(),
                                            options: ParamOptions {
                                                schema: Some(ty.as_ref().clone()),
                                                is_required: !arg.is_optional,
                                                ..Default::default()
                                            },
                                        },
                                        allow_reserved: false,
                                    });
                                }
                            }
                            _ => params.push(arg.as_query_param()?),
                        }
                    }
                }
                _ => if let Some(body_type) = self.arguments.get(params.len()) {
                    let schema: SchemaObject = body_type.ty.as_ref().try_into()?;
                    request_body = Some(
                        RequestBody::default().with_content("application/json", schema)
                    );
                }
            }
        }

        let response: Response = match SchemaObject::try_from(self.return_ty.as_ref()) {
            Ok(schema) => Response::default().with_content("application/json", schema),
            Err(ParseError::ObjectTypesShouldBeParsed) => {
                return Err(ParseError::ObjectTypesShouldBeParsed);
            }
            Err(ParseError::VectorTypesShouldBeParsed) => {
                return Err(ParseError::VectorTypesShouldBeParsed);
            }
            _ => { Response::default().with_description(Some("Success response")) }
        };

        let Some(path) = self.path.as_ref() else {
            return Err(ParseError::EndpointPathCannotBeNone);
        };

        const IGNORE_TAGS: [&str; 2] = ["Plugins", "$remoting-binding-module"];

        let tags = (
            match path.split('/').nth(1) {
                Some(segment) if path.starts_with("/lol-") => vec![format!("Plugin {}", segment)],
                Some(_) if path.starts_with("/{plugin}") =>
                    vec!["Plugin Static Assets".to_string()],
                Some(segment) => vec![segment.to_string()],
                None => {
                    eprintln!("Endpoint {} does not have a path", self.info.name);
                    vec![]
                }
            }
        )
            .into_iter()
            .chain(self.tags.iter().cloned())
            .dedup()
            .filter(|t| !IGNORE_TAGS.contains(&t.as_str()))
            .collect::<Vec<_>>();

        if tags.contains(&"builtin".to_string()) {
            println!("Endpoint {} has builtin tag, path: {}", self.info.name, path);
        }

        Ok(Operation {
            tags,
            description: Some(self.info.description.clone()),
            operation_id: Some(self.info.name.clone()),
            parameters: params,
            request_body,
            responses: HashMap::from_iter([("2XX".to_string(), response)]),
            ..Default::default()
        })
    }
}

impl help::Argument {
    /// Construct a [`PathParam::Query`] from the argument.
    ///
    /// Errors if the [`DataType`] cannot be converted to a [`SchemaObject`].
    pub fn as_query_param(&self) -> Result<Param, ParseError> {
        Ok(Param::Query {
            param: ParamSchema {
                name: self.info.name.clone(),
                style: ParamStyle::default_query(),
                options: ParamOptions {
                    schema: Some(SchemaObject::try_from(&self.ty)?),
                    is_required: !self.is_optional,
                    ..Default::default()
                },
            },
            allow_reserved: false,
        })
    }
}

impl PathItem {
    fn operations(&self) -> Vec<&Operation> {
        let mut ops = Vec::new();
        if let Some(op) = &self.get {
            ops.push(op);
        }
        if let Some(op) = &self.post {
            ops.push(op);
        }
        if let Some(op) = &self.put {
            ops.push(op);
        }
        if let Some(op) = &self.delete {
            ops.push(op);
        }
        if let Some(op) = &self.patch {
            ops.push(op);
        }
        if let Some(op) = &self.options {
            ops.push(op);
        }
        if let Some(op) = &self.head {
            ops.push(op);
        }
        if let Some(op) = &self.trace {
            ops.push(op);
        }
        ops
    }

    fn operations_mut(&mut self) -> Vec<&mut Operation> {
        let mut ops = Vec::new();
        if let Some(op) = &mut self.get {
            ops.push(op);
        }
        if let Some(op) = &mut self.post {
            ops.push(op);
        }
        if let Some(op) = &mut self.put {
            ops.push(op);
        }
        if let Some(op) = &mut self.delete {
            ops.push(op);
        }
        if let Some(op) = &mut self.patch {
            ops.push(op);
        }
        if let Some(op) = &mut self.options {
            ops.push(op);
        }
        if let Some(op) = &mut self.head {
            ops.push(op);
        }
        if let Some(op) = &mut self.trace {
            ops.push(op);
        }
        ops
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use ::serde::{ Serialize };

    use super::*;

    #[allow(dead_code)]
    /// Utility function to write a value to a file in pretty JSON format.
    fn w<T>(file_path: &str, value: T) -> Result<(), ParseError> where T: Serialize {
        let file = std::fs::File::create(file_path)?;
        let mut writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &value)?;
        writer.flush()?;
        drop(writer);
        Ok(())
    }

    #[allow(dead_code)]
    /// Utility function to read a value from a file in JSON format.
    fn r<T: ::serde::de::DeserializeOwned>(file_path: &str) -> Result<T, ParseError> {
        let file = std::fs::File::open(file_path)?;
        let reader = std::io::BufReader::new(file);
        let value = serde_json::from_reader(reader)?;
        Ok(value)
    }

    #[cfg(feature = "irelia")]
    #[tokio::test]
    #[ignore]
    async fn download_extended_help() {
        let mut lcu = lcu().unwrap();
        let xhelp = lcu.extended_help().await.unwrap();
        w("extended-help.json", xhelp).unwrap();
    }

    #[cfg(feature = "irelia")]
    #[tokio::test]
    #[ignore]
    async fn generate_openapi_v3() {
        let mut lcu = lcu().unwrap();
        let xhelp = r::<ExtendedHelp>("extended-help.json").unwrap();
        let spec = lcu.openapi(xhelp).await.unwrap();
        w("openapi.json", spec).unwrap();
    }

    #[cfg(feature = "irelia")]
    #[tokio::test]
    #[ignore]
    async fn generate_both() {
        let mut lcu = lcu().unwrap();
        let xhelp = lcu.extended_help().await.unwrap();
        let spec = lcu.openapi(xhelp.clone()).await.unwrap();
        w("extended-help.json", xhelp).unwrap();
        w("openapi.json", spec).unwrap();
    }

    #[cfg(feature = "irelia")]
    #[tokio::test]
    #[ignore]
    async fn general() -> Result<(), ParseError> {
        let spec = r::<OpenApiSpec>("openapi.json").unwrap();
        let mut hasagi = r::<OpenApiSpec>("hasagi-swagger.json").unwrap();
        hasagi.normalize_mut();

        // Need to patch some components in the hasagi spec to match our spec.
        let mut component_patches = HashMap::<_, Vec<(_, Option<_>)>>::default();

        // Define a macro to patch components
        macro_rules! patch {
            ($($name:expr => [$(($path:expr, $value:expr)),* $(,)?]),* $(,)?) => {
                $(
                    {
                        let entry = component_patches.entry($name.to_string()).or_default();
                        $(
                            entry.push(($path, $value));
                        )*
                    }
                )*
            };
        }

        // Patch components
        patch!(
            "LolLobbyLobbyChangeGameDto" => [
                ("properties.gameCustomization", Some(SchemaObject::object_of(SchemaObject::string()))),
                ("properties.customGameLobby", Some(SchemaObject::component_ref("LolLobbyLobbyCustomGameLobby")))
            ],
            "LolTftSkillTreeTftBattlepassInfo" => [
                ("properties.media", Some(SchemaObject::object_of(SchemaObject::string()))),
            ],
            "LolPatchPatchSieveReleaseInfo" => [
                ("properties.labels", Some(SchemaObject::object_of(SchemaObject::component_ref("LolPatchPatchSieveLabelValue")))),
            ],
            "ChemtechShoppe-FulfillmentDto" => [
                ("properties.displayMetadata", Some(SchemaObject::object_of(SchemaObject::object_of(true)))),
                ("properties.itemPayload", Some(SchemaObject::object_of(SchemaObject::object_of(true)))),
                ("properties.subCurrencyDeltas", Some(SchemaObject::object_of(SchemaObject::number("int64")))),
                ("properties.payload", None), // this is a private shop api, and poro_schema intentionally obscures it.
                ("properties.location", None), // deprecated api, or perhaps a todo.
                ("properties.target", None), // deprecated api, or perhaps a todo.
            ],
            "LolRankedEosNotificationsConfig" => [
                ("properties.config.items", Some(SchemaObject::component_ref("LolRankedEosNotificationsConfigEntry")))
            ]
        );

        let mut hasagi = serde_json::to_value(hasagi)?;
        for name in spec.components.keys() {
            if let Some(patches) = component_patches.remove(name) {
                println!("Patching {name}");
                for (path, value) in patches {
                    let path = format!("components.schemas.{name}.{path}");
                    hasagi.patch_mut(
                        path.as_str(),
                        value.map(|v| serde_json::to_value(v).unwrap())
                    )?;
                }
            }
        }

        let hasagi: OpenApiSpec = serde_json::from_value(hasagi)?;

        // ? Realized there's thousands of differences and they're all because we retain slightly more information
        // ? in our spec than hasagi does.
        /* // Compare the two components specs
        let mut diff = Vec::<String>::new();
        for (name, schema_obj) in spec.components.iter() {
            if let Some(other) = hasagi.components.schemas.get(name) {
                // compare for equality, ignoring the order of values in arrays
                let schema_obj = serde_json::to_value(schema_obj)?;
                let other = serde_json::to_value(other)?;
                let schema_obj = serde_json::from_value::<serde_json::Value>(schema_obj)?;
                let other = serde_json::from_value::<serde_json::Value>(other)?;
                if schema_obj != other {
                    diff.push(format!("{}: {schema_obj:#?} != {other:#?}", name));
                }
            } else {
                diff.push(format!("{}: {schema_obj:#?} not found in hasagi", name));
            }
        }

        if diff.len() > 0 {
            println!("Differences found: {}", diff.len());
            for d in diff.into_iter().take(1) {
                println!("{d}");
            }
        } else {
            println!("No differences found.");
        } */

        // ? Realized there's thousands of differences and they're all because we retain slightly more information
        // ? in our spec than hasagi does.
        /* // Compare the two paths specs
        let mut diff = Vec::<String>::new();
        for (name, path) in spec.paths.iter() {
            if let Some(other) = hasagi.paths.get(name) {
                // compare for equality, ignoring the order of values in arrays
                let path = serde_json::to_value(path)?;
                let other = serde_json::to_value(other)?;
                let path = serde_json::from_value::<serde_json::Value>(path)?;
                let other = serde_json::from_value::<serde_json::Value>(other)?;
                if path != other {
                    diff.push(format!("{}: {path:#?} != {other:#?}", name));
                }
            } else {
                diff.push(format!("{}: {path:#?} not found in hasagi", name));
            }
        }

        if diff.len() > 0 {
            println!("Path differences found: {}", diff.len());
            for d in diff.into_iter().take(1) {
                println!("{d}");
            }
        } else {
            println!("No differences found.");
        } */

        // Compare the two tag specs

        for (hasagi_tag, tag) in hasagi.tags.iter().zip(spec.tags.iter()) {
            if hasagi_tag != tag {
                println!("Tag mismatch: {} != {}", hasagi_tag.name, tag.name);
                println!("Hasagi tag: {:#?}", hasagi_tag);
                println!("Poro tag: {:#?}", tag);
            }
        }

        Ok(())
    }
}
