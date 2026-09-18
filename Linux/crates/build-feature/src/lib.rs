//! Rust port of `Packages/TexApp/Sources/BuildFeature`.

use build_core::{
    BuildCancellation, BuildID, BuildIssue, BuildLifecycle, BuildLogEntry, BuildToolTemplate,
    DirectCommandPlan, EnvironmentPolicy, LoginShellCommandPlan, ShellAuthority,
    ShellAuthoritySource, WorkingDirectoryPolicy,
};
use document_session_core::{DiskContentHash, DocumentSnapshot};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildFeatureError {
    NoCommandSelected,
    CustomShellAuthorityNotAcknowledged,
    BuildAlreadyActive,
    BuildIdentityMismatch,
    Plan(build_core::BuildCoreError),
}
impl fmt::Display for BuildFeatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for BuildFeatureError {}
impl From<build_core::BuildCoreError> for BuildFeatureError {
    fn from(e: build_core::BuildCoreError) -> Self {
        Self::Plan(e)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BuildDocumentRevision {
    pub document_id: String,
    pub revision: u64,
    pub content_hash: DiskContentHash,
}
impl BuildDocumentRevision {
    pub fn new(snapshot: &DocumentSnapshot) -> Self {
        Self {
            document_id: snapshot.document_id.raw_value().to_string(),
            revision: snapshot.revision,
            content_hash: snapshot.content_hash.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PendingCustomShellCommand {
    pub shell_executable: String,
    pub command: String,
    pub working_directory: WorkingDirectoryPolicy,
    pub environment: EnvironmentPolicy,
    pub source: ShellAuthoritySource,
    pub disclosure: String,
}
impl PendingCustomShellCommand {
    pub fn new(
        shell_executable: impl Into<String>,
        command: impl Into<String>,
        source: ShellAuthoritySource,
        disclosure: impl Into<String>,
    ) -> Self {
        Self {
            shell_executable: shell_executable.into(),
            command: command.into(),
            working_directory: WorkingDirectoryPolicy::ProjectRoot,
            environment: EnvironmentPolicy::Inherit {
                overrides: std::collections::HashMap::new(),
            },
            source,
            disclosure: disclosure.into(),
        }
    }

    fn acknowledging_authority(&self) -> Result<LoginShellCommandPlan, build_core::BuildCoreError> {
        let authority = ShellAuthority::new(self.source, true, &self.disclosure)?;
        LoginShellCommandPlan::new(
            &self.shell_executable,
            &self.command,
            self.working_directory.clone(),
            self.environment.clone(),
            authority,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildCommandSelection {
    BuiltIn(BuildToolTemplate),
    Direct(DirectCommandPlan),
    CustomShellAwaitingAcknowledgement(PendingCustomShellCommand),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResolvedBuildCommand {
    Direct(DirectCommandPlan),
    LoginShell(LoginShellCommandPlan),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildRunState {
    Idle,
    Active {
        id: BuildID,
        lifecycle: BuildLifecycle,
        cancellation: Option<BuildCancellation>,
    },
}

#[derive(Debug)]
pub struct BuildFeatureState {
    selection: Option<BuildCommandSelection>,
    authorized_custom_shell: Option<LoginShellCommandPlan>,
    run: BuildRunState,
    input_revision: Option<BuildDocumentRevision>,
    log: Vec<BuildLogEntry>,
    issues: Vec<BuildIssue>,
}
impl Default for BuildFeatureState {
    fn default() -> Self {
        Self::new()
    }
}
impl BuildFeatureState {
    pub fn new() -> Self {
        Self {
            selection: None,
            authorized_custom_shell: None,
            run: BuildRunState::Idle,
            input_revision: None,
            log: Vec::new(),
            issues: Vec::new(),
        }
    }
    pub fn selection(&self) -> Option<&BuildCommandSelection> {
        self.selection.as_ref()
    }
    pub fn authorized_custom_shell(&self) -> Option<&LoginShellCommandPlan> {
        self.authorized_custom_shell.as_ref()
    }
    pub fn run(&self) -> &BuildRunState {
        &self.run
    }
    pub fn input_revision(&self) -> Option<&BuildDocumentRevision> {
        self.input_revision.as_ref()
    }
    pub fn log(&self) -> &[BuildLogEntry] {
        &self.log
    }
    pub fn issues(&self) -> &[BuildIssue] {
        &self.issues
    }

    pub fn select(&mut self, selection: BuildCommandSelection) {
        self.selection = Some(selection);
        self.authorized_custom_shell = None;
    }

    pub fn acknowledge_custom_shell_authority(&mut self) -> Result<(), BuildFeatureError> {
        match &self.selection {
            Some(BuildCommandSelection::CustomShellAwaitingAcknowledgement(pending)) => {
                self.authorized_custom_shell = Some(pending.acknowledging_authority()?);
                Ok(())
            }
            _ => Err(BuildFeatureError::CustomShellAuthorityNotAcknowledged),
        }
    }

    pub fn resolved_command(&self, input: &str) -> Result<ResolvedBuildCommand, BuildFeatureError> {
        match self.selection.as_ref() {
            None => Err(BuildFeatureError::NoCommandSelected),
            Some(BuildCommandSelection::BuiltIn(template)) => {
                Ok(ResolvedBuildCommand::Direct(template.plan(input)?))
            }
            Some(BuildCommandSelection::Direct(plan)) => {
                Ok(ResolvedBuildCommand::Direct(plan.clone()))
            }
            Some(BuildCommandSelection::CustomShellAwaitingAcknowledgement(_)) => {
                match &self.authorized_custom_shell {
                    Some(plan) => Ok(ResolvedBuildCommand::LoginShell(plan.clone())),
                    None => Err(BuildFeatureError::CustomShellAuthorityNotAcknowledged),
                }
            }
        }
    }

    pub fn queue(
        &mut self,
        build_id: BuildID,
        input: &DocumentSnapshot,
    ) -> Result<(), BuildFeatureError> {
        if self.run != BuildRunState::Idle {
            return Err(BuildFeatureError::BuildAlreadyActive);
        }
        self.resolved_command(input.path.raw_value())?;
        self.run = BuildRunState::Active {
            id: build_id,
            lifecycle: BuildLifecycle::Queued,
            cancellation: None,
        };
        self.input_revision = Some(BuildDocumentRevision::new(input));
        self.log.clear();
        self.issues.clear();
        Ok(())
    }

    pub fn transition(
        &mut self,
        build_id: &BuildID,
        to: BuildLifecycle,
    ) -> Result<(), BuildFeatureError> {
        match &self.run {
            BuildRunState::Active { id, lifecycle, cancellation } if id == build_id => {
                let next = lifecycle.transitioning(to)?;
                self.run = BuildRunState::Active {
                    id: id.clone(),
                    lifecycle: next,
                    cancellation: cancellation.clone(),
                };
                Ok(())
            }
            _ => Err(BuildFeatureError::BuildIdentityMismatch),
        }
    }

    pub fn request_cancellation(
        &mut self,
        build_id: &BuildID,
        cancellation: BuildCancellation,
    ) -> Result<(), BuildFeatureError> {
        match &self.run {
            BuildRunState::Active { id, lifecycle, .. } if id == build_id => {
                let next = lifecycle.transitioning(BuildLifecycle::Cancelling)?;
                self.run = BuildRunState::Active {
                    id: id.clone(),
                    lifecycle: next,
                    cancellation: Some(cancellation),
                };
                Ok(())
            }
            _ => Err(BuildFeatureError::BuildIdentityMismatch),
        }
    }

    pub fn record_log(&mut self, entry: BuildLogEntry) {
        self.log.retain(|e| e.sequence != entry.sequence);
        self.log.push(entry);
        self.log.sort_by_key(|e| e.sequence);
    }

    pub fn replace_issues(&mut self, issues: Vec<BuildIssue>) {
        let mut sorted = issues;
        sorted.sort_by(|a, b| {
            (
                a.file.clone().unwrap_or_default(),
                a.line.unwrap_or(0),
                a.message.clone(),
            )
                .cmp(&(
                    b.file.clone().unwrap_or_default(),
                    b.line.unwrap_or(0),
                    b.message.clone(),
                ))
        });
        self.issues = sorted;
    }

    pub fn reset_after_completion(&mut self) {
        if let BuildRunState::Active { lifecycle, .. } = &self.run {
            match lifecycle {
                BuildLifecycle::Succeeded { .. }
                | BuildLifecycle::Failed { .. }
                | BuildLifecycle::Cancelled { .. } => self.run = BuildRunState::Idle,
                _ => {}
            }
        }
    }
}
