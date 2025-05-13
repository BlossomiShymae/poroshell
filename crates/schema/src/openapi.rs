use serde::{ Deserialize, Serialize };
use serde_json::Value;

#[derive(Deserialize, Serialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: OpenApiInfo,
    pub components: serde_json::Map<String, Value>,
    pub paths: serde_json::Map<String, Value>,
}

#[derive(Deserialize, Serialize)]
pub struct OpenApiInfo {
    pub title: String,
    pub description: String,
    pub version: String,
}
