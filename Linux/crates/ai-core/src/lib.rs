//! Rust port of `Packages/TexCore/Sources/AICore` — shared AI value types,
//! state machines, markdown splitting, and revision-bound edit proposals.
//! No transport lives here.

use document_session_core::{DiskContentHash, DocumentSnapshot};
use serde::{Deserialize, Serialize};
use std::fmt;
use tex_domain::StableDocumentID;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AICoreError {
    EmptyValue,
    InvalidURL,
    InvalidTransition,
    ContextLimitExceeded { maximum_utf8_bytes: usize },
    RetryLimitExceeded,
    Cancelled,
    InvalidEditRange,
}
impl fmt::Display for AICoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for AICoreError {}

fn validated_opaque(value: &str, minimum_length: usize) -> Result<String, AICoreError> {
    if value.len() < minimum_length || value.chars().any(|c| c == '\0') {
        return Err(AICoreError::EmptyValue);
    }
    Ok(value.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AIRole {
    System,
    User,
    Assistant,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContextMessage {
    pub role: AIRole,
    pub content: String,
}
impl ContextMessage {
    pub fn new(role: AIRole, content: &str) -> Result<Self, AICoreError> {
        Ok(Self {
            role,
            content: validated_opaque(content, 1)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AIContext {
    pub messages: Vec<ContextMessage>,
    #[serde(rename = "maximumUTF8Bytes")]
    pub maximum_utf8_bytes: usize,
}
impl AIContext {
    pub fn new(
        messages: Vec<ContextMessage>,
        maximum_utf8_bytes: usize,
    ) -> Result<Self, AICoreError> {
        if maximum_utf8_bytes == 0 {
            return Err(AICoreError::ContextLimitExceeded {
                maximum_utf8_bytes,
            });
        }
        let total: usize = messages.iter().map(|m| m.content.len()).sum();
        if total > maximum_utf8_bytes {
            return Err(AICoreError::ContextLimitExceeded {
                maximum_utf8_bytes,
            });
        }
        Ok(Self {
            messages,
            maximum_utf8_bytes,
        })
    }
    pub fn appending(&self, message: ContextMessage) -> Result<Self, AICoreError> {
        let mut messages = self.messages.clone();
        messages.push(message);
        Self::new(messages, self.maximum_utf8_bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StreamEvent {
    MarkdownDelta(String),
    Usage {
        #[serde(rename = "inputUnits")]
        input_units: i64,
        #[serde(rename = "outputUnits")]
        output_units: i64,
    },
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StreamState {
    Idle,
    Streaming { markdown: String },
    Completed { markdown: String },
    Failed(String),
    Cancelled,
}
impl StreamState {
    pub fn consuming(&self, event: &StreamEvent) -> Result<Self, AICoreError> {
        match (self, event) {
            (StreamState::Idle, StreamEvent::MarkdownDelta(delta)) => Ok(StreamState::Streaming {
                markdown: delta.clone(),
            }),
            (StreamState::Streaming { markdown }, StreamEvent::MarkdownDelta(delta)) => {
                Ok(StreamState::Streaming {
                    markdown: format!("{markdown}{delta}"),
                })
            }
            (StreamState::Streaming { markdown }, StreamEvent::Usage { .. }) => {
                Ok(StreamState::Streaming {
                    markdown: markdown.clone(),
                })
            }
            (StreamState::Streaming { markdown }, StreamEvent::Completed) => {
                Ok(StreamState::Completed {
                    markdown: markdown.clone(),
                })
            }
            _ => Err(AICoreError::InvalidTransition),
        }
    }
    pub fn failing(&self, message: &str) -> Result<Self, AICoreError> {
        if !matches!(self, StreamState::Streaming { .. }) {
            return Err(AICoreError::InvalidTransition);
        }
        Ok(StreamState::Failed(validated_opaque(message, 1)?))
    }
    pub fn cancelling(&self) -> Result<Self, AICoreError> {
        if !matches!(self, StreamState::Streaming { .. }) {
            return Err(AICoreError::InvalidTransition);
        }
        Ok(StreamState::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RetryPolicy {
    #[serde(rename = "maximumAttempts")]
    pub maximum_attempts: i64,
    #[serde(rename = "baseDelayMilliseconds")]
    pub base_delay_milliseconds: u64,
    #[serde(rename = "maximumDelayMilliseconds")]
    pub maximum_delay_milliseconds: u64,
}
impl RetryPolicy {
    pub fn new(
        maximum_attempts: i64,
        base_delay_milliseconds: u64,
        maximum_delay_milliseconds: u64,
    ) -> Result<Self, AICoreError> {
        if maximum_attempts <= 0
            || maximum_attempts > 10
            || base_delay_milliseconds > maximum_delay_milliseconds
        {
            return Err(AICoreError::RetryLimitExceeded);
        }
        Ok(Self {
            maximum_attempts,
            base_delay_milliseconds,
            maximum_delay_milliseconds,
        })
    }
    pub fn delay(&self, before_attempt: i64) -> Result<u64, AICoreError> {
        if before_attempt <= 1 || before_attempt > self.maximum_attempts {
            return Err(AICoreError::RetryLimitExceeded);
        }
        let shift = (before_attempt - 2).min(62) as u32;
        let multiplied = self
            .base_delay_milliseconds
            .checked_mul(1u64 << shift)
            .unwrap_or(u64::MAX);
        Ok(multiplied.min(self.maximum_delay_milliseconds))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CancellationToken {
    #[serde(rename = "isCancelled")]
    pub is_cancelled: bool,
}
impl CancellationToken {
    pub fn new(is_cancelled: bool) -> Self {
        Self { is_cancelled }
    }
    pub fn checked(&self) -> Result<(), AICoreError> {
        if self.is_cancelled {
            return Err(AICoreError::Cancelled);
        }
        Ok(())
    }
    pub fn cancelling(&self) -> Self {
        Self {
            is_cancelled: true,
        }
    }
}
impl Default for CancellationToken {
    fn default() -> Self {
        Self::new(false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarkdownBlock {
    Prose(String),
    FencedCode { language: Option<String>, code: String },
}

pub struct MarkdownParser;
impl MarkdownParser {
    pub fn blocks(markdown: &str) -> Vec<MarkdownBlock> {
        markdown
            .split("```")
            .enumerate()
            .filter_map(|(index, part)| {
                if part.is_empty() {
                    return None;
                }
                if index % 2 == 0 {
                    return Some(MarkdownBlock::Prose(part.to_string()));
                }
                let mut lines = part.split('\n');
                let first = lines.next().unwrap_or("");
                let language = if first.is_empty() {
                    None
                } else {
                    Some(first.to_string())
                };
                Some(MarkdownBlock::FencedCode {
                    language,
                    code: lines.collect::<Vec<_>>().join("\n"),
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextEdit {
    #[serde(rename = "utf8Offset")]
    pub utf8_offset: usize,
    #[serde(rename = "utf8Length")]
    pub utf8_length: usize,
    pub replacement: String,
}
impl TextEdit {
    pub fn new(
        utf8_offset: usize,
        utf8_length: usize,
        replacement: impl Into<String>,
    ) -> Result<Self, AICoreError> {
        let replacement = replacement.into();
        Ok(Self {
            utf8_offset,
            utf8_length,
            replacement,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EditProposalState {
    Proposed { id: String, edits: Vec<TextEdit> },
    Accepted { id: String, edits: Vec<TextEdit> },
    Rejected { id: String },
    Applied { id: String },
}
impl EditProposalState {
    pub fn accepting(&self) -> Result<Self, AICoreError> {
        match self {
            Self::Proposed { id, edits } if !edits.is_empty() => Ok(Self::Accepted {
                id: id.clone(),
                edits: edits.clone(),
            }),
            _ => Err(AICoreError::InvalidTransition),
        }
    }
    pub fn rejecting(&self) -> Result<Self, AICoreError> {
        match self {
            Self::Proposed { id, .. } => Ok(Self::Rejected { id: id.clone() }),
            _ => Err(AICoreError::InvalidTransition),
        }
    }
    pub fn applied(&self) -> Result<Self, AICoreError> {
        match self {
            Self::Accepted { id, .. } => Ok(Self::Applied { id: id.clone() }),
            _ => Err(AICoreError::InvalidTransition),
        }
    }
}

// ---------------------------------------------------------------------------
// RevisionBoundEdit.swift

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevisionBoundEditError {
    EmptyProposalID,
    EmptyEdits,
    InvalidRange,
    UnorderedOrOverlappingEdits,
    OutOfBounds,
    MidScalarBoundary,
    DisclosureNotApproved,
    EmptyDisclosure,
    ImplicitWholeDocumentDisclosure,
    DocumentMismatch {
        expected: StableDocumentID,
        actual: StableDocumentID,
    },
    StaleRevision { expected: u64, actual: u64 },
    ContentHashMismatch {
        expected: DiskContentHash,
        actual: DiskContentHash,
    },
    ApplyRequiresAcceptedProposal,
    InvalidTransition,
}
impl fmt::Display for RevisionBoundEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for RevisionBoundEditError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UTF8TextRange {
    pub offset: usize,
    pub length: usize,
}
impl UTF8TextRange {
    pub fn new(offset: usize, length: usize) -> Result<Self, RevisionBoundEditError> {
        if offset.checked_add(length).is_none() {
            return Err(RevisionBoundEditError::InvalidRange);
        }
        Ok(Self { offset, length })
    }
    pub fn upper_bound(&self) -> usize {
        self.offset + self.length
    }
}

/// Describes exactly which source text the user permits an AI provider to receive.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AIContextDisclosureScope {
    SelectedRanges(Vec<UTF8TextRange>),
    WholeDocument {
        #[serde(rename = "userApproved")]
        user_approved: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevisionBoundEditProposal {
    pub proposal_id: String,
    pub document_id: StableDocumentID,
    pub base_revision: u64,
    pub base_content_hash: DiskContentHash,
    pub edits: Vec<TextEdit>,
    pub disclosure_scope: AIContextDisclosureScope,
}
impl RevisionBoundEditProposal {
    pub fn new(
        proposal_id: impl Into<String>,
        snapshot: &DocumentSnapshot,
        edits: Vec<TextEdit>,
        disclosure_scope: AIContextDisclosureScope,
    ) -> Result<Self, RevisionBoundEditError> {
        let proposal_id = proposal_id.into();
        if proposal_id.is_empty() || proposal_id.chars().any(|c| c == '\0') {
            return Err(RevisionBoundEditError::EmptyProposalID);
        }
        if edits.is_empty()
            || !edits
                .iter()
                .all(|e| e.utf8_length > 0 || !e.replacement.is_empty())
        {
            return Err(RevisionBoundEditError::EmptyEdits);
        }
        let actual_content_hash = DiskContentHash::hashing(&snapshot.text);
        if snapshot.content_hash != actual_content_hash {
            return Err(RevisionBoundEditError::ContentHashMismatch {
                expected: snapshot.content_hash,
                actual: actual_content_hash,
            });
        }
        let edit_ranges: Vec<UTF8TextRange> = edits
            .iter()
            .map(|e| UTF8TextRange::new(e.utf8_offset, e.utf8_length))
            .collect::<Result<_, _>>()?;
        Self::validate_ordered_ranges(&edit_ranges, &snapshot.text)?;
        Self::validate_disclosure(&disclosure_scope, &snapshot.text)?;

        Ok(Self {
            proposal_id,
            document_id: snapshot.document_id.clone(),
            base_revision: snapshot.revision,
            base_content_hash: snapshot.content_hash,
            edits,
            disclosure_scope,
        })
    }

    fn validate_base(&self, snapshot: &DocumentSnapshot) -> Result<(), RevisionBoundEditError> {
        if snapshot.document_id != self.document_id {
            return Err(RevisionBoundEditError::DocumentMismatch {
                expected: self.document_id.clone(),
                actual: snapshot.document_id.clone(),
            });
        }
        if snapshot.revision != self.base_revision {
            return Err(RevisionBoundEditError::StaleRevision {
                expected: self.base_revision,
                actual: snapshot.revision,
            });
        }
        let actual = DiskContentHash::hashing(&snapshot.text);
        if snapshot.content_hash != self.base_content_hash || actual != self.base_content_hash {
            return Err(RevisionBoundEditError::ContentHashMismatch {
                expected: self.base_content_hash,
                actual,
            });
        }
        Ok(())
    }

    fn validate_disclosure(
        scope: &AIContextDisclosureScope,
        text: &str,
    ) -> Result<(), RevisionBoundEditError> {
        match scope {
            AIContextDisclosureScope::WholeDocument { user_approved } => {
                if !user_approved {
                    return Err(RevisionBoundEditError::DisclosureNotApproved);
                }
            }
            AIContextDisclosureScope::SelectedRanges(ranges) => {
                if ranges.is_empty() || !ranges.iter().any(|r| r.length > 0) {
                    return Err(RevisionBoundEditError::EmptyDisclosure);
                }
                Self::validate_ordered_ranges(ranges, text)?;
                if ranges.first().map(|r| r.offset) == Some(0)
                    && ranges.last().map(|r| r.upper_bound()) == Some(text.len())
                    && ranges
                        .iter()
                        .zip(ranges.iter().skip(1))
                        .all(|(a, b)| a.upper_bound() == b.offset)
                {
                    return Err(RevisionBoundEditError::ImplicitWholeDocumentDisclosure);
                }
            }
        }
        Ok(())
    }

    fn validate_ordered_ranges(
        ranges: &[UTF8TextRange],
        text: &str,
    ) -> Result<(), RevisionBoundEditError> {
        let mut previous_upper_bound = 0usize;
        for (index, range) in ranges.iter().enumerate() {
            if range.upper_bound() > text.len() {
                return Err(RevisionBoundEditError::OutOfBounds);
            }
            if index > 0 && range.offset < previous_upper_bound {
                return Err(RevisionBoundEditError::UnorderedOrOverlappingEdits);
            }
            if !Self::is_scalar_boundary(range.offset, text)
                || !Self::is_scalar_boundary(range.upper_bound(), text)
            {
                return Err(RevisionBoundEditError::MidScalarBoundary);
            }
            previous_upper_bound = range.upper_bound();
        }
        Ok(())
    }

    /// Scalar boundary = UTF-8 code-point boundary (Swift's `text.utf8[i] &
    /// 0b1100_0000 != 0b1000_0000`).
    fn is_scalar_boundary(offset: usize, text: &str) -> bool {
        if offset > text.len() {
            return false;
        }
        if offset == text.len() {
            return true;
        }
        text.as_bytes()[offset] & 0b1100_0000 != 0b1000_0000
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NativeUndoTransactionDescriptor {
    #[serde(rename = "proposalID")]
    pub proposal_id: String,
    #[serde(rename = "documentID")]
    pub document_id: StableDocumentID,
    #[serde(rename = "baseRevision")]
    pub base_revision: u64,
    #[serde(rename = "actionName")]
    pub action_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevisionBoundEditApplication {
    pub transaction: RevisionBoundEditTransaction,
    pub replacement_text: String,
    pub undo_transaction: NativeUndoTransactionDescriptor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevisionBoundEditTransaction {
    pub proposal: RevisionBoundEditProposal,
    pub state: EditProposalState,
}
impl RevisionBoundEditTransaction {
    pub fn new(proposal: RevisionBoundEditProposal) -> Self {
        Self {
            state: EditProposalState::Proposed {
                id: proposal.proposal_id.clone(),
                edits: proposal.edits.clone(),
            },
            proposal,
        }
    }
    fn with_state(proposal: RevisionBoundEditProposal, state: EditProposalState) -> Self {
        Self { proposal, state }
    }

    pub fn accepting(&self) -> Result<Self, RevisionBoundEditError> {
        match &self.state {
            EditProposalState::Proposed { id, edits }
                if *id == self.proposal.proposal_id && *edits == self.proposal.edits => {}
            _ => return Err(RevisionBoundEditError::InvalidTransition),
        }
        Ok(Self::with_state(
            self.proposal.clone(),
            self.state
                .accepting()
                .map_err(|_| RevisionBoundEditError::InvalidTransition)?,
        ))
    }

    pub fn rejecting(&self) -> Result<Self, RevisionBoundEditError> {
        match &self.state {
            EditProposalState::Proposed { id, edits }
                if *id == self.proposal.proposal_id && *edits == self.proposal.edits => {}
            _ => return Err(RevisionBoundEditError::InvalidTransition),
        }
        Ok(Self::with_state(
            self.proposal.clone(),
            self.state
                .rejecting()
                .map_err(|_| RevisionBoundEditError::InvalidTransition)?,
        ))
    }

    pub fn applying(
        &self,
        snapshot: &DocumentSnapshot,
    ) -> Result<RevisionBoundEditApplication, RevisionBoundEditError> {
        match &self.state {
            EditProposalState::Accepted { id, edits }
                if *id == self.proposal.proposal_id && *edits == self.proposal.edits => {}
            _ => return Err(RevisionBoundEditError::ApplyRequiresAcceptedProposal),
        }
        self.proposal.validate_base(snapshot)?;

        let source = snapshot.text.as_bytes();
        let mut replacement: Vec<u8> = Vec::with_capacity(source.len());
        let mut cursor = 0usize;
        for edit in &self.proposal.edits {
            let upper_bound = edit.utf8_offset + edit.utf8_length;
            replacement.extend_from_slice(&source[cursor..edit.utf8_offset]);
            replacement.extend_from_slice(edit.replacement.as_bytes());
            cursor = upper_bound;
        }
        replacement.extend_from_slice(&source[cursor..]);
        let replacement_text = String::from_utf8_lossy(&replacement).into_owned();

        let applied = Self::with_state(
            self.proposal.clone(),
            self.state
                .applied()
                .map_err(|_| RevisionBoundEditError::InvalidTransition)?,
        );
        Ok(RevisionBoundEditApplication {
            transaction: applied,
            replacement_text,
            undo_transaction: NativeUndoTransactionDescriptor {
                proposal_id: self.proposal.proposal_id.clone(),
                document_id: self.proposal.document_id.clone(),
                base_revision: self.proposal.base_revision,
                action_name: "Apply AI Edit".to_string(),
            },
        })
    }
}
