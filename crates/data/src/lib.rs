use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;

pub type Plugins = BTreeMap<String, Vec<Plugin>>;

#[derive(Debug, Clone)]
pub struct Document {
    plugins: Plugins,
    info: openapi::types::Info,
    paths: Vec<String>,
}

impl Document {
    pub fn new(data: openapi::types::Document) -> Self {
        let mut plugins = Plugins::new();

        for (path, path_item) in data.paths.iter() {
            for (method, operation) in path_item {
                let mut subplugins = Vec::<Plugin>::new();
                let mut key = String::from("_unknown");

                // Process and group endpoints into the following formats:
                // "_unknown" - group that should not be possible
                // "default" - no tags
                // "builtin" - 'builtin' not associated with an endpoint
                // "lol-summoner" etc. - 'plugin' associated with an endpoint
                // "performance", "tracing", etc.
                if operation.tags.is_empty() {
                    key = String::from("default");
                    if plugins.contains_key(&key) {
                        let _ = plugins
                            .get_mut(&key)
                            .map(|p| p.push(Plugin::new(&method, &path, &key, &operation)));
                    } else {
                        subplugins.push(Plugin::new(&method, &path, &key, &operation));
                        plugins.insert(String::from(key), subplugins);
                    }
                } else {
                    for tag in operation.tags.iter() {
                        if tag == "plugins" {
                            continue;
                        } else {
                            key = tag.clone();
                        }

                        if plugins.contains_key(&key) {
                            let _ = plugins
                                .get_mut(&key)
                                .map(|p| p.push(Plugin::new(&method, &path, &tag, &operation)));
                        } else {
                            subplugins.push(Plugin::new(&method, &path, &tag, &operation));
                            plugins.insert(key, subplugins.clone());
                        }
                    }
                }
            }
        }

        let paths = data.paths.keys().cloned().collect::<Vec<String>>();

        Self {
            plugins,
            info: data.info,
            paths,
        }
    }

    pub fn plugins(&self) -> Plugins {
        self.plugins.clone()
    }

    pub fn info(&self) -> openapi::types::Info {
        self.info.clone()
    }

    pub fn paths(&self) -> Vec<String> {
        self.paths.clone()
    }
}

#[derive(Debug, Clone)]
pub struct Plugin {
    method: String,
    path: String,
    tag: String,
    operation: openapi::types::Operation,
}

impl Plugin {
    pub fn new(method: &str, path: &str, tag: &str, operation: &openapi::types::Operation) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            tag: tag.into(),
            operation: operation.clone(),
        }
    }

    pub fn method(&self) -> String {
        self.method.clone()
    }

    pub fn path(&self) -> String {
        self.path.clone()
    }

    pub fn tag(&self) -> String {
        self.tag.clone()
    }

    pub fn operation(&self) -> openapi::types::Operation {
        self.operation.clone()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct RiotAPILibrary {
    pub owner: String,
    pub repo: String,
    pub language: String,
    pub tags: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn it_works() {}
}
