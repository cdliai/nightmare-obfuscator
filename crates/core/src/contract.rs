use crate::{NightmareError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const RUN_CONTRACT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunConfig {
    pub schema_version: u16,
    pub source: PathBuf,
    pub output: PathBuf,
    #[serde(default)]
    pub profile: RunProfile,
    #[serde(default = "default_intensity")]
    pub intensity: u8,
    #[serde(default)]
    pub selected_paths: Vec<PathBuf>,
    #[serde(default)]
    pub ignored_patterns: Vec<String>,
    #[serde(default)]
    pub owner: OwnerConfig,
    #[serde(default)]
    pub project: ProjectConfig,
    #[serde(default)]
    pub checks: CheckConfig,
    #[serde(default)]
    pub features: FeatureConfig,
    #[serde(default)]
    pub signing: SigningConfig,
}

impl RunConfig {
    pub fn template(source: PathBuf, output: PathBuf, owner: String, project: String) -> Self {
        Self {
            schema_version: RUN_CONTRACT_SCHEMA_VERSION,
            source,
            output,
            profile: RunProfile::Balanced,
            intensity: default_intensity(),
            selected_paths: Vec::new(),
            ignored_patterns: Vec::new(),
            owner: OwnerConfig {
                name: owner,
                contact: None,
            },
            project: ProjectConfig { name: project },
            checks: CheckConfig::default(),
            features: FeatureConfig::default(),
            signing: SigningConfig::default(),
        }
    }

    pub fn from_toml_file(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&text)
            .map_err(|err| NightmareError::Config(format!("invalid run contract TOML: {err}")))?;
        config.validate()?;
        Ok(config)
    }

    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string_pretty(self).map_err(|err| {
            NightmareError::Config(format!("could not serialize run contract: {err}"))
        })
    }

    pub fn resolve_relative_paths(&mut self, base: &Path) {
        if self.source.is_relative() {
            self.source = base.join(&self.source);
        }
        if self.output.is_relative() {
            self.output = base.join(&self.output);
        }
        if let Some(private_key_path) = &mut self.signing.private_key_path {
            if private_key_path.is_relative() {
                *private_key_path = base.join(&private_key_path);
            }
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.schema_version != RUN_CONTRACT_SCHEMA_VERSION {
            return Err(NightmareError::Config(format!(
                "unsupported schema_version {}; expected {}",
                self.schema_version, RUN_CONTRACT_SCHEMA_VERSION
            )));
        }
        if self.intensity == 0 || self.intensity > 10 {
            return Err(NightmareError::Config(
                "intensity must be between 1 and 10".to_string(),
            ));
        }
        if self.owner.name.trim().is_empty() {
            return Err(NightmareError::Config("owner.name is required".to_string()));
        }
        if self.project.name.trim().is_empty() {
            return Err(NightmareError::Config(
                "project.name is required".to_string(),
            ));
        }
        if self.source == self.output {
            return Err(NightmareError::Config(
                "source and output must be different paths".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunProfile {
    Light,
    #[default]
    Balanced,
    Aggressive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerConfig {
    pub name: String,
    #[serde(default)]
    pub contact: Option<String>,
}

impl Default for OwnerConfig {
    fn default() -> Self {
        Self {
            name: "unclaimed".to_string(),
            contact: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub name: String,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "project".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckConfig {
    #[serde(default = "default_true")]
    pub verify_metadata: bool,
    #[serde(
        default = "default_build_check",
        deserialize_with = "deserialize_build_check",
        serialize_with = "serialize_build_check"
    )]
    pub build: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureConfig {
    #[serde(default = "default_true")]
    pub dead_code: bool,
    /// Experimental and not yet implemented: currently a no-op that does not
    /// transform control flow. Disabled by default to avoid implying a
    /// protection that is not applied. See `docs/run-contract.md`.
    #[serde(default)]
    pub flatten_control_flow: bool,
    #[serde(default)]
    pub encrypt_strings: bool,
    #[serde(default = "default_true")]
    pub rename_identifiers: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SigningConfig {
    #[serde(default)]
    pub private_key_path: Option<PathBuf>,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            dead_code: true,
            flatten_control_flow: false,
            encrypt_strings: false,
            rename_identifiers: true,
        }
    }
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self {
            verify_metadata: true,
            build: default_build_check(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub schema_version: u16,
    pub status: RunStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_path: Option<PathBuf>,
    pub source: PathBuf,
    pub output: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_path: Option<PathBuf>,
    pub stages: Vec<RunStage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Passed,
    Failed,
    Planned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStage {
    pub name: String,
    pub status: StageStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_path: Option<PathBuf>,
}

impl RunStage {
    pub fn passed(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: StageStatus::Passed,
            command: Some(command.into()),
            exit_code: Some(0),
            stderr_summary: None,
            manifest_path: None,
        }
    }

    pub fn failed(name: impl Into<String>, command: impl Into<String>, err: impl ToString) -> Self {
        Self {
            name: name.into(),
            status: StageStatus::Failed,
            command: Some(command.into()),
            exit_code: None,
            stderr_summary: Some(summarize(err.to_string())),
            manifest_path: None,
        }
    }

    pub fn skipped(
        name: impl Into<String>,
        command: impl Into<String>,
        reason: impl ToString,
    ) -> Self {
        Self {
            name: name.into(),
            status: StageStatus::Skipped,
            command: Some(command.into()),
            exit_code: None,
            stderr_summary: Some(summarize(reason.to_string())),
            manifest_path: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StageStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateContext {
    pub provider: String,
    pub repo: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub disposable_workspace: PathBuf,
    pub mcp_adapter: String,
}

fn default_intensity() -> u8 {
    7
}

fn default_true() -> bool {
    true
}

fn default_build_check() -> Option<String> {
    Some("cargo test".to_string())
}

fn deserialize_build_check<'de, D>(deserializer: D) -> std::result::Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = toml::Value::deserialize(deserializer)?;
    match value {
        toml::Value::Boolean(false) => Ok(None),
        toml::Value::Boolean(true) => Ok(default_build_check()),
        toml::Value::String(command) if !command.trim().is_empty() => Ok(Some(command)),
        toml::Value::String(_) => Err(serde::de::Error::custom(
            "checks.build must be false or a non-empty command string",
        )),
        _ => Err(serde::de::Error::custom(
            "checks.build must be false or a command string",
        )),
    }
}

fn serialize_build_check<S>(
    build: &Option<String>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match build {
        Some(command) => serializer.serialize_str(command),
        None => serializer.serialize_bool(false),
    }
}

fn summarize(value: String) -> String {
    let trimmed = value.trim();
    let mut summary = trimmed.chars().take(600).collect::<String>();
    if trimmed.chars().count() > 600 {
        summary.push_str("...");
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_control_flow_defaults_off() {
        // flatten_control_flow is an unimplemented no-op; it must stay disabled
        // by default and when omitted from the contract so a run never implies a
        // control-flow transform that did not happen.
        assert!(!FeatureConfig::default().flatten_control_flow);

        let minimal = r#"
schema_version = 1
source = "./in"
output = "./out"

[owner]
name = "CDLI"

[project]
name = "demo"
"#;
        let config: RunConfig = toml::from_str(minimal).expect("minimal contract parses");
        assert!(!config.features.flatten_control_flow);
    }
}
