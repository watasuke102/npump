use anyhow::{Context, Result, bail};

use crate::{model::WorkspaceState, registry::Registry, workspace};

pub struct App {
    pub workspaces: Vec<WorkspaceState>,
    pub active_workspace: usize,
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

        Ok(Self {
            workspaces,
            active_workspace: 0,
            status,
            should_quit: false,
        })
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
