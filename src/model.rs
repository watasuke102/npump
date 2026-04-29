use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use semver::Version;
use serde_json::{Map, Value};

static SEMVER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?").unwrap());

#[derive(Clone, Copy, Debug)]
pub enum DependencySection {
    Dependencies,
    DevDependencies,
    PeerDependencies,
    OptionalDependencies,
}

impl DependencySection {
    pub fn all() -> [Self; 4] {
        [
            Self::Dependencies,
            Self::DevDependencies,
            Self::PeerDependencies,
            Self::OptionalDependencies,
        ]
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Dependencies => "dependencies",
            Self::DevDependencies => "devDependencies",
            Self::PeerDependencies => "peerDependencies",
            Self::OptionalDependencies => "optionalDependencies",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dependencies => "dep",
            Self::DevDependencies => "dev",
            Self::PeerDependencies => "peer",
            Self::OptionalDependencies => "opt",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PackageEntry {
    pub name: String,
    pub section: DependencySection,
    pub current_spec: String,
    pub current_version: Version,
    pub latest_version: Version,
    pub selected: bool,
}

impl PackageEntry {
    pub fn updated_spec(&self) -> String {
        let replacement = self.latest_version.to_string();
        SEMVER_RE
            .replacen(&self.current_spec, 1, replacement.as_str())
            .to_string()
    }
}

#[derive(Debug)]
pub struct WorkspaceState {
    pub tab_name: String,
    pub package_path: PathBuf,
    pub package_json: Value,
    pub entries: Vec<PackageEntry>,
    pub selected_index: usize,
}

impl WorkspaceState {
    pub fn move_selection_down(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.entries.len();
    }

    pub fn move_selection_up(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        self.selected_index = if self.selected_index == 0 {
            self.entries.len() - 1
        } else {
            self.selected_index - 1
        };
    }

    pub fn toggle_selected(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        if let Some(entry) = self.entries.get_mut(self.selected_index) {
            entry.selected = !entry.selected;
        }
    }

    pub fn toggle_all(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let should_select_all = self.entries.iter().any(|entry| !entry.selected);
        for entry in &mut self.entries {
            entry.selected = should_select_all;
        }
    }

    pub fn apply_selected_updates(&mut self) -> Result<usize> {
        let selected: Vec<(DependencySection, String, String)> = self
            .entries
            .iter()
            .filter(|entry| entry.selected)
            .map(|entry| (entry.section, entry.name.clone(), entry.updated_spec()))
            .collect();
        if selected.is_empty() {
            return Ok(0);
        }
        let updated_count = selected.len();

        for (section, package_name, spec) in selected {
            if self.package_json.get(section.key()).is_none() {
                self.package_json[section.key()] = Value::Object(Map::new());
            }
            if let Some(deps) = self
                .package_json
                .get_mut(section.key())
                .and_then(Value::as_object_mut)
            {
                deps.insert(package_name, Value::String(spec));
            }
        }

        let payload = serde_json::to_string_pretty(&self.package_json)
            .context("failed to serialize package.json")?;
        fs::write(&self.package_path, format!("{payload}\n"))
            .with_context(|| format!("failed to write {}", self.package_path.display()))?;

        self.entries.retain(|entry| !entry.selected);
        if self.entries.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.entries.len() {
            self.selected_index = self.entries.len() - 1;
        }

        Ok(updated_count)
    }
}

pub fn extract_first_semver(spec: &str) -> Option<Version> {
    let matched = SEMVER_RE.find(spec.trim())?;
    Version::parse(matched.as_str()).ok()
}
