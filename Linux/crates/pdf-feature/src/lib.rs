//! Rust port of `Packages/TexApp/Sources/PDFFeature`.

use app_ports::{PDFCoordinateSpace, PDFPoint};
use document_session_core::DiskContentHash;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use synctex_core::{
    ExactMatchSelector, ForwardSyncQuery, InverseSyncQuery, NormalizedSourcePath, SyncTeXError,
    SyncTeXMatch, SyncTeXRevision,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PDFPreviewArtifact {
    pub path: NormalizedSourcePath,
    pub sync_revision: SyncTeXRevision,
    pub source_content_hash: DiskContentHash,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PDFPreviewState {
    Unavailable,
    Loading(PDFPreviewArtifact),
    Ready { artifact: PDFPreviewArtifact, page_count: i64, visible_page: i64 },
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncTeXNavigationRequest {
    Forward(ForwardSyncQuery),
    Inverse(InverseSyncQuery),
}
impl SyncTeXNavigationRequest {
    pub fn revision(&self) -> &SyncTeXRevision {
        match self {
            Self::Forward(q) => &q.revision,
            Self::Inverse(q) => &q.revision,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncTeXNavigationState {
    Idle,
    Verifying(SyncTeXNavigationRequest),
    Matched(SyncTeXMatch),
    Stale { requested: SyncTeXRevision, available: Vec<SyncTeXRevision> },
    Ambiguous { match_count: usize },
    NoMatch,
    Failed { message: String },
}

#[derive(Debug)]
pub struct PDFFeatureState {
    preview: PDFPreviewState,
    navigation: SyncTeXNavigationState,
}
impl Default for PDFFeatureState {
    fn default() -> Self {
        Self::new()
    }
}
impl PDFFeatureState {
    pub fn new() -> Self {
        Self { preview: PDFPreviewState::Unavailable, navigation: SyncTeXNavigationState::Idle }
    }
    pub fn preview(&self) -> &PDFPreviewState {
        &self.preview
    }
    pub fn navigation(&self) -> &SyncTeXNavigationState {
        &self.navigation
    }

    pub fn begin_loading(&mut self, artifact: PDFPreviewArtifact) {
        self.preview = PDFPreviewState::Loading(artifact);
        self.navigation = SyncTeXNavigationState::Idle;
    }

    pub fn finish_loading(
        &mut self,
        artifact: PDFPreviewArtifact,
        page_count: i64,
        visible_page: i64,
    ) {
        if page_count <= 0 || !(1..=page_count).contains(&visible_page) {
            self.preview =
                PDFPreviewState::Failed { message: "The generated PDF has invalid page metadata.".into() };
            return;
        }
        self.preview = PDFPreviewState::Ready { artifact, page_count, visible_page };
    }

    pub fn fail_loading(&mut self, message: &str) {
        self.preview = PDFPreviewState::Failed { message: message.to_string() };
        self.navigation = SyncTeXNavigationState::Idle;
    }

    pub fn show_page(&mut self, page: i64) {
        if let PDFPreviewState::Ready { artifact, page_count, .. } = &self.preview {
            if (1..=*page_count).contains(&page) {
                self.preview = PDFPreviewState::Ready {
                    artifact: artifact.clone(),
                    page_count: *page_count,
                    visible_page: page,
                };
            }
        }
    }

    pub fn verify_forward(
        &mut self,
        query: ForwardSyncQuery,
        candidates: &[SyncTeXMatch],
    ) -> Option<SyncTeXMatch> {
        self.navigation = SyncTeXNavigationState::Verifying(SyncTeXNavigationRequest::Forward(query.clone()));
        self.verify(&query.revision, candidates, |c| ExactMatchSelector::forward(c, &query))
    }

    pub fn verify_inverse(
        &mut self,
        query: InverseSyncQuery,
        candidates: &[SyncTeXMatch],
    ) -> Option<SyncTeXMatch> {
        self.navigation = SyncTeXNavigationState::Verifying(SyncTeXNavigationRequest::Inverse(query.clone()));
        self.verify(&query.revision, candidates, |c| ExactMatchSelector::inverse(c, &query))
    }

    fn verify(
        &mut self,
        query_revision: &SyncTeXRevision,
        candidates: &[SyncTeXMatch],
        selector: impl FnOnce(&[SyncTeXMatch]) -> Result<SyncTeXMatch, SyncTeXError>,
    ) -> Option<SyncTeXMatch> {
        let available: Vec<SyncTeXRevision> = {
            let set: BTreeSet<(String, u64)> = candidates
                .iter()
                .map(|m| (m.revision.build_id.clone(), m.revision.fingerprint))
                .collect();
            set.into_iter()
                .map(|(build_id, fingerprint)| SyncTeXRevision { build_id, fingerprint })
                .collect()
        };
        if !candidates.is_empty() && !candidates.iter().any(|m| &m.revision == query_revision) {
            self.navigation = SyncTeXNavigationState::Stale {
                requested: query_revision.clone(),
                available,
            };
            return None;
        }

        match selector(candidates) {
            Ok(m) => {
                self.navigation = SyncTeXNavigationState::Matched(m.clone());
                Some(m)
            }
            Err(SyncTeXError::NoMatch) => {
                self.navigation = SyncTeXNavigationState::NoMatch;
                None
            }
            Err(SyncTeXError::AmbiguousMatch { count }) => {
                self.navigation = SyncTeXNavigationState::Ambiguous { match_count: count };
                None
            }
            Err(SyncTeXError::StaleResult) => {
                self.navigation = SyncTeXNavigationState::Stale {
                    requested: query_revision.clone(),
                    available,
                };
                None
            }
            Err(e) => {
                self.navigation = SyncTeXNavigationState::Failed { message: format!("{e:?}") };
                None
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PDFPointConversionRequest {
    pub point: PDFPoint,
    pub source: PDFCoordinateSpace,
    pub destination: PDFCoordinateSpace,
}
