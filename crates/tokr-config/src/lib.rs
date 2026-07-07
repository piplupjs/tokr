use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TokrConfig {
    #[serde(rename = "$schema")]
    #[serde(default)]
    pub schema: Option<String>,
    pub input: Vec<String>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub options: ConfigOptions,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HexCase {
    #[default]
    Lower,
    Upper,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LintConfig {
    #[serde(rename = "kebab-case-vars")]
    #[serde(default)]
    pub kebab_case_vars: Option<bool>,

    #[serde(rename = "hex-case")]
    pub hex_case: Option<HexCase>,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Ts,
    Js,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfigOptions {
    #[serde(default)]
    pub strict: bool,
    #[serde(default)]
    pub format: OutputFormat,
    #[serde(default)]
    pub declaration: bool,
    #[serde(default)]
    pub order: Option<HashMap<String, Vec<String>>>,
    #[serde(rename = "allowNameMismatch")]
    #[serde(default)]
    pub allow_name_mismatch: bool,
    #[serde(default)]
    pub lint: LintConfig,
}

impl TokrConfig {
    pub fn load(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config() {
        let json = r#"{
            "input": ["src/**/*.scss"],
            "output": "theme.ts",
            "options": {
                "strict": true,
                "allowNameMismatch": true,
                "order": {
                    "": ["colors", "typography"]
                }
            }
        }"#;

        let config = TokrConfig::load(json).unwrap();
        assert_eq!(config.input, vec!["src/**/*.scss"]);
        assert_eq!(config.output, Some("theme.ts".to_string()));
        assert!(config.options.strict);
        assert!(config.options.allow_name_mismatch);
        assert_eq!(
            config.options.order.unwrap().get(""),
            Some(&vec!["colors".to_string(), "typography".to_string()])
        );
    }

    #[test]
    fn test_missing_input() {
        let json = r#"{
            "output": "theme.ts"
        }"#;

        let err = TokrConfig::load(json).unwrap_err();
        assert!(err.to_string().contains("missing field `input`"));
    }

    #[test]
    fn test_unknown_field() {
        let json = r#"{
            "input": ["src/**/*.scss"],
            "unknown": "field"
        }"#;

        let err = TokrConfig::load(json).unwrap_err();
        assert!(err.to_string().contains("unknown field `unknown`"));
    }

    #[test]
    fn test_unknown_option() {
        let json = r#"{
            "input": ["src/**/*.scss"],
            "options": {
                "foo": "bar"
            }
        }"#;

        let err = TokrConfig::load(json).unwrap_err();
        assert!(err.to_string().contains("unknown field `foo`"));
    }
}
