use irelia::{ requests::RequestClientTrait, rest::LcuClient };

use error::Error;
use openapi::{ OpenApiInfo, OpenApiSpec };
use patch::Patch;
use help::{ ConsoleEndpointInner, Endpoint, Event, ExtendedHelp, Help, SeqFirst, Type };

/// `use poro_schema::prelude::*;` to import common traits and types.
pub mod prelude {
    pub use super::{ lcu, PoroSchema, help::ExtendedHelp, openapi::OpenApiSpec };
}

pub mod help;
pub mod error;
pub mod openapi;
pub mod patch;

/// Pattern: `apply_patches!(to: $jsons, name_lens: $name_lens, patches: [ ($name, $($path, $value),*), ... ])`
macro_rules! apply_patches {
    (to: $jsons:expr, patches: [$(($name:expr, $(($path:expr, $value:expr)),* $(,)?)),* $(,)?]) => {
        {
            for json in $jsons.iter_mut() {
                
                $(
                    if json.get("name").and_then(|v| v.as_str()).unwrap() == $name {
                        $(
                            json.patch_mut($path, $value.into())?;
                        )*
                    }
                )*
            }
        }
    };
}

/// Create a new irelia client.
#[inline]
pub fn lcu() -> Result<LcuClient<irelia::requests::RequestClientType>, Error> {
    let req = irelia::requests::new();
    let lcu = LcuClient::connect_with_request_client(&req)?;
    Ok(lcu)
}

pub trait PoroSchema {
    /// Construct [`ExtendedHelp`] using the LCU API.
    fn extended_help(
        &self
    ) -> impl std::future::Future<Output = Result<ExtendedHelp, Error>> + Send;

    /// Construct [`OpenApiSpec`] using the LCU API.
    fn openapi(&self) -> impl std::future::Future<Output = Result<OpenApiSpec, Error>> + Send;

    // /// Construct [`Swagger`] using the LCU API.
    // async fn swagger(&self) -> Result<Swagger, Error>;
}

impl<T: RequestClientTrait + Clone> PoroSchema
    for LcuClient<T>
    where error::Error: From<irelia::error::Error<<T as RequestClientTrait>::Error>>
{
    async fn extended_help(&self) -> Result<ExtendedHelp, Error> {
        let help: Help = self.post("/help", "").await?;

        // construct the extended help object
        let mut full_types = Vec::<Type>::new();
        let mut full_events = Vec::<Event>::new();
        let mut full_endpoints = Vec::<serde_json::Value>::new();

        // Get help for all types
        for ty_name in help.types.keys() {
            let endpoint = format!("/help?target={ty_name}&format=Full");
            let SeqFirst::<Type>(full) = self.post(endpoint, "").await?;
            full_types.push(full);
        }

        // Get help for all events
        for ev_name in help.events.keys() {
            let endpoint = format!("/help?target={ev_name}&format=Full");
            let SeqFirst::<Event>(full) = self.post(endpoint, "").await?;
            full_events.push(full);
        }

        // Get help for all endpoints
        let reg = regex::Regex::new(r"\{(.*?)\}");
        for fn_name in help.functions.keys() {
            let endpoint = format!("/help?target={fn_name}&format=Full");
            let SeqFirst::<Endpoint>(mut full) = self.post(endpoint, "").await?;

            // Finish construction using data from console help.
            {
                let endpoint = format!("/help?target={fn_name}&format=Console");
                let mut console: serde_json::Value = self.post(endpoint, "").await?;
                let console = console
                    .as_object_mut()
                    .expect("Console endpoint response should be an object");

                if let Some(console) = console.remove(fn_name) {
                    let console: ConsoleEndpointInner = serde_json::from_value(console)?;
                    full.path_params = if let Some(url) = console.url.as_ref() {
                        reg.clone()
                            .unwrap()
                            .captures_iter(url.as_str())
                            .map(|cap| cap[1].to_string())
                            .collect::<Vec<_>>()
                    } else {
                        Vec::new()
                    };
                    full.path = console.url;
                    full.method = console.http_method;
                }
            }
            let full = serde_json::to_value(full)?;
            full_endpoints.push(full);
        }

        // Apply endpoint patches
        apply_patches!(
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
                match serde_json::from_value::<Endpoint>(json) {
                    Ok(endpoint) => endpoint,
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

    async fn openapi(&self) -> Result<OpenApiSpec, Error> {
        use serde_json::Value;
        use serde::{ Deserialize, Serialize };

        #[derive(Deserialize)]
        struct Version {
            version: String,
        }
        let Version { version } = self.get("/system/v1/builds").await?;

        let mut endpoints_with_missing_data = Vec::<String>::new();

        let mut spec = OpenApiSpec {
            openapi: "3.0.0".to_string(),
            info: OpenApiInfo {
                title: "LCU PORO-SCHEMA".to_string(),
                description: "OpenAPI v3 specification for LCU".to_string(),
                version,
            },
            components: serde_json::Map::new(),
            paths: serde_json::Map::new(),
        };

        Ok(spec)
    }
}

pub fn swagger() -> Result<(), Error> {
    todo!()
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn w<T>(name: &str, value: T) -> Result<(), Error> where T: serde::Serialize {
        let file = std::fs::File::create(name)?;
        let mut writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &value)?;
        writer.flush()?;
        drop(writer);
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn download_extended_help() {
        let lcu = lcu().unwrap();
        let xhelp = lcu.extended_help().await.unwrap();
        w("extended-help.json", xhelp).unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn generate_openapi_v3() {
        let lcu = lcu().unwrap();
        lcu.openapi().await.unwrap();
    }

    #[test]
    fn test_apply_patches_macro() -> Result<(), Error> {
        let mut jsons = vec![
            serde_json::json!({ "name": "Help", "method": "get", "path": "/help" }),
            serde_json::json!({ "name": "Subscribe", "method": "get", "path": "/subscribe" })
        ];

        apply_patches!(
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
