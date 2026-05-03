use anyhow::{Context, Result, bail};

use crate::{model::WorkspaceState, registry::Registry, workspace};

pub struct App {
    pub workspaces: Vec<WorkspaceState>,
    pub active_workspace: usize,
    pub package_name_width: usize,
    pub version_width: usize,
    pub status: String,
    pub should_quit: bool,
}

impl App {
    pub fn load() -> Result<Self> {
        let cwd = std::env::current_dir().context("failed to read current directory")?;
        let mut registry = Registry::new()?;
        let workspaces = workspace::load_workspaces(&cwd, &mut registry)?;
        if workspaces.is_empty() {
            bail!("no package.json found");
        }

        let mut status = String::from("Ready");
        let warnings = registry.take_warnings();
        if let Some(first) = warnings.first() {
            status = format!("Ready with {} warning(s): {first}", warnings.len());
        }

        let (package_name_width, version_width) = Self::calculate_column_widths(&workspaces);

        Ok(Self {
            workspaces,
            active_workspace: 0,
            package_name_width,
            version_width,
            status,
            should_quit: false,
        })
    }

    fn calculate_column_widths(workspaces: &[WorkspaceState]) -> (usize, usize) {
        let package_name_width = workspaces
            .iter()
            .flat_map(|workspace| workspace.entries.iter())
            .map(|entry| entry.name.chars().count())
            .max()
            .unwrap_or(1);
        let version_width = workspaces
            .iter()
            .flat_map(|workspace| workspace.entries.iter())
            .flat_map(|entry| {
                [
                    entry.current_version.to_string().chars().count(),
                    entry.latest_version.to_string().chars().count(),
                ]
            })
            .max()
            .unwrap_or(1);
        (package_name_width, version_width)
    }

    pub fn refresh_column_widths(&mut self) {
        (self.package_name_width, self.version_width) = Self::calculate_column_widths(&self.workspaces);
    }

    pub fn current_workspace(&self) -> &WorkspaceState {
        &self.workspaces[self.active_workspace]
    }

    pub fn current_workspace_mut(&mut self) -> &mut WorkspaceState {
        &mut self.workspaces[self.active_workspace]
    }

    pub fn next_workspace(&mut self) {
        self.active_workspace = (self.active_workspace + 1) % self.workspaces.len();
    }

    pub fn previous_workspace(&mut self) {
        self.active_workspace = if self.active_workspace == 0 {
            self.workspaces.len() - 1
        } else {
            self.active_workspace - 1
        };
    }

    pub fn move_selection_down(&mut self) {
        self.current_workspace_mut().move_selection_down();
    }

    pub fn move_selection_up(&mut self) {
        self.current_workspace_mut().move_selection_up();
    }

    pub fn toggle_selected(&mut self) {
        self.current_workspace_mut().toggle_selected();
    }

    pub fn toggle_all(&mut self) {
        self.current_workspace_mut().toggle_all();
    }

    pub fn apply_current_workspace_updates(&mut self) -> Result<usize> {
        self.current_workspace_mut().apply_selected_updates()
    }
}
