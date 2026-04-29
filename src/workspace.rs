use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use glob::glob;
use serde_json::Value;

use crate::{
    model::{DependencySection, PackageEntry, WorkspaceState, extract_first_semver},
    registry::Registry,
};

pub fn load_workspaces(root: &Path, registry: &mut Registry) -> Result<Vec<WorkspaceState>> {
    println!("Loading workspaces from {}", root.display());
    let root_package_json = root.join("package.json");
    if !root_package_json.exists() {
        bail!("package.json was not found in {}", root.display());
    }

    let root_body = fs::read_to_string(&root_package_json)
        .with_context(|| format!("failed to read {}", root_package_json.display()))?;
    let root_json: Value = serde_json::from_str(&root_body)
        .with_context(|| format!("failed to parse {}", root_package_json.display()))?;

    let mut package_paths = HashSet::<PathBuf>::new();
    if let Some(workspaces) = root_json.get("workspaces") {
        let patterns: Vec<String> = if let Some(array) = workspaces.as_array() {
            array
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        } else if let Some(array) = workspaces
            .as_object()
            .and_then(|map| map.get("packages"))
            .and_then(Value::as_array)
        {
            array
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        } else {
            Vec::new()
        };

        for pattern in patterns {
            let absolute_pattern = root.join(&pattern).to_string_lossy().to_string();
            for entry in glob(&absolute_pattern)
                .with_context(|| format!("invalid workspace pattern '{pattern}'"))?
            {
                let path = match entry {
                    Ok(path) => path,
                    Err(_) => continue,
                };
                let candidate = if path.is_dir() {
                    path.join("package.json")
                } else if path.file_name().and_then(|name| name.to_str()) == Some("package.json") {
                    path
                } else {
                    path.join("package.json")
                };
                if candidate.exists() {
                    package_paths.insert(candidate);
                }
            }
        }
    }

    let seeds: Vec<(PathBuf, Option<String>)> = if package_paths.is_empty() {
        let fallback_name = root
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| String::from("workspace"));
        vec![(root_package_json, Some(fallback_name))]
    } else {
        let mut sorted_paths: Vec<PathBuf> = package_paths.into_iter().collect();
        sorted_paths.sort();
        sorted_paths
            .into_iter()
            .map(|package_path| (package_path, None))
            .collect()
    };

    let mut workspaces = Vec::with_capacity(seeds.len());
    for (package_path, tab_override) in seeds {
        let body = fs::read_to_string(&package_path)
            .with_context(|| format!("failed to read {}", package_path.display()))?;
        let package_json: Value = serde_json::from_str(&body)
            .with_context(|| format!("failed to parse {}", package_path.display()))?;

        let tab_name = tab_override.unwrap_or_else(|| {
            package_json
                .get("name")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .or_else(|| {
                    package_path
                        .parent()
                        .and_then(Path::file_name)
                        .and_then(|name| name.to_str())
                        .map(ToString::to_string)
                })
                .unwrap_or_else(|| String::from("workspace"))
        });

        let mut entries = Vec::new();
        for section in DependencySection::all() {
            let Some(deps) = package_json.get(section.key()).and_then(Value::as_object) else {
                continue;
            };
            for (name, spec_value) in deps {
                let Some(spec) = spec_value.as_str() else {
                    continue;
                };
                let normalized_spec = spec.trim();
                if [
                    "workspace:",
                    "file:",
                    "link:",
                    "git+",
                    "http://",
                    "https://",
                    "github:",
                    "npm:",
                ]
                .iter()
                .any(|prefix| normalized_spec.starts_with(prefix))
                {
                    continue;
                }
                let Some(current_version) = extract_first_semver(normalized_spec) else {
                    continue;
                };
                let Some(latest_version) = registry.latest(name) else {
                    continue;
                };
                if latest_version <= current_version {
                    continue;
                }
                entries.push(PackageEntry {
                    name: name.to_string(),
                    section,
                    current_spec: spec.to_string(),
                    current_version,
                    latest_version,
                    selected: false,
                });
            }
        }
        entries.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then_with(|| a.section.key().cmp(b.section.key()))
        });

        workspaces.push(WorkspaceState {
            tab_name,
            package_path,
            package_json,
            entries,
            selected_index: 0,
        });
    }

    Ok(workspaces)
}
