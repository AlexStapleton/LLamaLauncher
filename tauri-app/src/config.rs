use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArgumentOverride {
    pub flag: String,
    pub values: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub schema_version: u32,
    pub llama_cpp_path: String,
    pub models_path: String,
    pub model_file: String,
    pub host: String,
    pub port: u16,
    pub argument_overrides: Vec<ArgumentOverride>,
    pub additional_args: Vec<String>,
    pub router_mode: bool,
    pub mcp_profile: crate::mcp::Profile,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: 2,
            llama_cpp_path: String::new(),
            models_path: String::new(),
            model_file: String::new(),
            host: "127.0.0.1".into(),
            port: 8080,
            argument_overrides: Vec::new(),
            additional_args: Vec::new(),
            router_mode: false,
            mcp_profile: crate::mcp::Profile::default(),
        }
    }
}
