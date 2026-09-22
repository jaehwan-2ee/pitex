//! Rust port of `Packages/TexApp/Sources/EditorFeature`.

pub mod fold;

use ai_core::{EditProposalState, NativeUndoTransactionDescriptor, RevisionBoundEditApplication};
use app_ports::{DocumentMutation, DocumentTextRange};
use language_core::{LanguageTokenKind, SourceRange};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorFeatureError {
    InvalidRange,
    SelectionOutsideDocument,
    StaleDecorationRevision { expected: u64, received: u64 },
    StaleMutationRevision { expected: u64, received: u64 },
    InvalidAITransaction,
}
impl fmt::Display for EditorFeatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for EditorFeatureError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorTextRange {
    pub location: i64,
    pub length: i64,
}
impl EditorTextRange {
    pub fn new(location: i64, length: i64) -> Result<Self, EditorFeatureError> {
        if location < 0 || length < 0 || location > i64::MAX - length {
            return Err(EditorFeatureError::InvalidRange);
        }
        Ok(Self { location, length })
    }
    pub fn upper_bound(&self) -> i64 {
        self.location + self.length
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorSelection {
    pub ranges: Vec<EditorTextRange>,
    pub primary_range_index: usize,
}
impl EditorSelection {
    pub fn new(
        ranges: Vec<EditorTextRange>,
        primary_range_index: usize,
    ) -> Result<Self, EditorFeatureError> {
        if ranges.is_empty() || primary_range_index >= ranges.len() {
            return Err(EditorFeatureError::InvalidRange);
        }
        Ok(Self { ranges, primary_range_index })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarkedTextPresentation {
    pub range: EditorTextRange,
    pub selected_range: EditorTextRange,
}
impl MarkedTextPresentation {
    pub fn new(
        range: EditorTextRange,
        selected_range: EditorTextRange,
    ) -> Result<Self, EditorFeatureError> {
        if selected_range.upper_bound() > range.length {
            return Err(EditorFeatureError::InvalidRange);
        }
        Ok(Self { range, selected_range })
    }
}

/// Immutable UI metadata for a core-owned document revision; it intentionally
/// does not own text.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorPresentationSnapshot {
    pub document_revision: u64,
    pub utf16_length: i64,
    pub selection: EditorSelection,
    pub marked_text: Option<MarkedTextPresentation>,
}
impl EditorPresentationSnapshot {
    pub fn new(
        document_revision: u64,
        utf16_length: i64,
        selection: EditorSelection,
        marked_text: Option<MarkedTextPresentation>,
    ) -> Result<Self, EditorFeatureError> {
        if utf16_length < 0
            || !selection.ranges.iter().all(|r| r.upper_bound() <= utf16_length)
            || marked_text.map(|m| m.range.upper_bound() <= utf16_length).unwrap_or(true) == false
        {
            return Err(EditorFeatureError::SelectionOutsideDocument);
        }
        Ok(Self { document_revision, utf16_length, selection, marked_text })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorDecoration {
    pub range: SourceRange,
    pub token_kind: LanguageTokenKind,
}

/// Stable per-variant tag name — the enum payloads carry token text, so the
/// Debug format would produce a different name per token. Both the adapter
/// (apply) and the shell (color) key tags through this name.
pub fn decoration_tag_name(kind: &LanguageTokenKind) -> &'static str {
    use LanguageTokenKind as K;
    match kind {
        K::ControlSequence(_) => "pitex.decoration.controlsequence",
        K::Comment(_) => "pitex.decoration.comment",
        K::LeftBrace => "pitex.decoration.leftbrace",
        K::RightBrace => "pitex.decoration.rightbrace",
        K::Whitespace(_) => "pitex.decoration.whitespace",
        K::Text(_) => "pitex.decoration.text",
        K::BibEntryMarker => "pitex.decoration.bibentrymarker",
        K::Punctuation(_) => "pitex.decoration.punctuation",
        K::EnvironmentName(_) => "pitex.decoration.environmentname",
        K::Math(_) => "pitex.decoration.math",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorDecorationSnapshot {
    pub document_revision: u64,
    pub decorations: Vec<EditorDecoration>,
}
impl EditorDecorationSnapshot {
    pub fn new(document_revision: u64, mut decorations: Vec<EditorDecoration>) -> Self {
        decorations.sort_by(|a, b| {
            a.range
                .utf8_offset
                .cmp(&b.range.utf8_offset)
                .then(a.range.utf8_length.cmp(&b.range.utf8_length))
        });
        Self { document_revision, decorations }
    }
    pub fn checked(
        &self,
        presentation: &EditorPresentationSnapshot,
    ) -> Result<&[EditorDecoration], EditorFeatureError> {
        if self.document_revision != presentation.document_revision {
            return Err(EditorFeatureError::StaleDecorationRevision {
                expected: presentation.document_revision,
                received: self.document_revision,
            });
        }
        Ok(&self.decorations)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EditorUndoPolicy {
    Register { action_name: String },
    Coalesce { identifier: String },
    DoNotRegister,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorMutationIntent {
    pub base_revision: u64,
    pub range: EditorTextRange,
    pub replacement: String,
    pub undo_policy: EditorUndoPolicy,
}
impl EditorMutationIntent {
    pub fn checked(
        &self,
        presentation: &EditorPresentationSnapshot,
    ) -> Result<DocumentMutation, EditorFeatureError> {
        if self.base_revision != presentation.document_revision {
            return Err(EditorFeatureError::StaleMutationRevision {
                expected: presentation.document_revision,
                received: self.base_revision,
            });
        }
        if self.range.upper_bound() > presentation.utf16_length {
            return Err(EditorFeatureError::SelectionOutsideDocument);
        }
        Ok(DocumentMutation {
            base_revision: self.base_revision,
            range: DocumentTextRange {
                location: self.range.location as usize,
                length: self.range.length as usize,
            },
            replacement: self.replacement.clone(),
        })
    }
}

/// An explicit user intent emitted only after a revision-bound AI proposal was
/// accepted and applied.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EditorAIApplyIntent {
    pub proposal_id: String,
    pub base_revision: u64,
    pub replacement_text: String,
    pub undo_transaction: NativeUndoTransactionDescriptor,
}
impl EditorAIApplyIntent {
    pub fn new(
        application: &RevisionBoundEditApplication,
    ) -> Result<Self, EditorFeatureError> {
        let proposal = &application.transaction.proposal;
        let applied_ok = matches!(
            &application.transaction.state,
            EditProposalState::Applied { id } if *id == proposal.proposal_id
        );
        if !applied_ok
            || proposal.proposal_id != application.undo_transaction.proposal_id
            || proposal.base_revision != application.undo_transaction.base_revision
        {
            return Err(EditorFeatureError::InvalidAITransaction);
        }
        Ok(Self {
            proposal_id: proposal.proposal_id.clone(),
            base_revision: proposal.base_revision,
            replacement_text: application.replacement_text.clone(),
            undo_transaction: application.undo_transaction.clone(),
        })
    }

    pub fn checked(
        &self,
        presentation: &EditorPresentationSnapshot,
    ) -> Result<EditorMutationIntent, EditorFeatureError> {
        if self.base_revision != presentation.document_revision {
            return Err(EditorFeatureError::StaleMutationRevision {
                expected: presentation.document_revision,
                received: self.base_revision,
            });
        }
        Ok(EditorMutationIntent {
            base_revision: self.base_revision,
            range: EditorTextRange::new(0, presentation.utf16_length)?,
            replacement: self.replacement_text.clone(),
            undo_policy: EditorUndoPolicy::Register {
                action_name: self.undo_transaction.action_name.clone(),
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EditorUserIntent {
    Mutate(EditorMutationIntent),
    ApplyAI(EditorAIApplyIntent),
    SetSelection(EditorSelection),
    SetMarkedText(Option<MarkedTextPresentation>),
    Undo,
    Redo,
}
