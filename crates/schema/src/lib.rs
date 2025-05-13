use irelia::rest::LcuClient;

use error::Error;
use patch::Patch;
use help::{ ConsoleEndpointInner, Endpoint, Event, ExtendedHelp, Help, SeqFirst, Type };

mod help;
mod error;
mod patch;

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

/// Construct [`ExtendedHelp`] using the LCU API.
pub async fn extended_help() -> Result<ExtendedHelp, Error> {
    let req = irelia::requests::new();
    let lcu = LcuClient::connect_with_request_client(&req)?;

    let help: Help = lcu.post("/help", "").await?;

    // construct the extended help object
    let mut full_types = Vec::<Type>::new();
    let mut full_events = Vec::<Event>::new();
    let mut full_endpoints = Vec::<serde_json::Value>::new();

    // Get help for all types
    for ty_name in help.types.keys().take(10) {
        let endpoint = format!("/help?target={}&format=Full", ty_name);
        let SeqFirst::<Type>(full) = lcu.post(endpoint, "").await?;
        full_types.push(full);
    }

    // Get help for all events
    for ev_name in help.events.keys().take(10) {
        let endpoint = format!("/help?target={}&format=Full", ev_name);
        let SeqFirst::<Event>(full) = lcu.post(endpoint, "").await?;
        full_events.push(full);
    }

    // Get help for all endpoints
    for fn_name in help.functions.keys().take(10) {
        let endpoint = format!("/help?target={}&format=Full", fn_name);
        let SeqFirst::<Endpoint>(mut full) = lcu.post(endpoint, "").await?;

        // Finish construction using data from console help.
        {
            let endpoint = format!("/help?target={}&format=Console", fn_name);
            let mut console: serde_json::Value = lcu.post(endpoint, "").await?;
            let console = console
                .as_object_mut()
                .expect("Console endpoint response should be an object");

            if let Some(console) = console.remove(fn_name) {
                let console: ConsoleEndpointInner = serde_json::from_value(console)?;
                full.path_params = if let Some(url) = console.url.as_ref() {
                    regex::Regex
                        ::new(r"\{(.*?)\}")
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
        .filter_map(|json| {
            match serde_json::from_value::<Endpoint>(json) {
                Ok(endpoint) => Some(endpoint),
                Err(e) => {
                    panic!("Something major changed in the API. Please report this error: {}", e);
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
        let xhelp = extended_help().await.unwrap();
        w("extended-help.json", xhelp).unwrap();
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
