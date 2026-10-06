//! Rust port of `Packages/TexCore/Sources/ParityKit/ParityKit.swift` —
//! manifest/coverage/evidence validation for the macOS↔Linux parity gate.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParityValidationError {
    InvalidStableID,
    UpdaterExclusionRequired,
    UpdaterExclusionForbidden,
    InvalidChecksum,
    MissingEvidence(String),
    StaleEvidence(String),
    ProvisionalCoverage(String),
    DuplicateID(String),
}
impl fmt::Display for ParityValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ParityValidationError {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ParityStableID {
    pub raw_value: String,
}
impl ParityStableID {
    pub fn new(raw_value: impl Into<String>) -> Result<Self, ParityValidationError> {
        let raw_value = raw_value.into();
        let bytes = raw_value.as_bytes();
        if bytes.is_empty()
            || bytes.len() > 128
            || !bytes[0].is_ascii_lowercase()
            || !bytes
                .iter()
                .all(|b| matches!(b, b'-' | b'.' | b'_') || b.is_ascii_lowercase() || b.is_ascii_digit())
        {
            return Err(ParityValidationError::InvalidStableID);
        }
        Ok(Self { raw_value })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ManifestTaxonomy {
    ApplicationShell,
    DocumentEditing,
    ProjectManagement,
    Typesetting,
    Bibliography,
    PdfPreview,
    SyncTeX,
    ArtificialIntelligence,
    Preferences,
    Accessibility,
    Updater,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExclusionReason {
    UpdaterOutOfScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ParityDisposition {
    Required,
    Excluded { reason: ExclusionReason },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ManifestItem {
    pub id: ParityStableID,
    pub taxonomy: ManifestTaxonomy,
    pub title: String,
    pub disposition: ParityDisposition,
}
impl ManifestItem {
    pub fn new(
        id: ParityStableID,
        taxonomy: ManifestTaxonomy,
        title: impl Into<String>,
        disposition: ParityDisposition,
    ) -> Result<Self, ParityValidationError> {
        let title = title.into();
        if title.is_empty() {
            return Err(ParityValidationError::InvalidStableID);
        }
        if taxonomy == ManifestTaxonomy::Updater {
            if disposition
                != (ParityDisposition::Excluded {
                    reason: ExclusionReason::UpdaterOutOfScope,
                })
            {
                return Err(ParityValidationError::UpdaterExclusionRequired);
            }
        } else if matches!(disposition, ParityDisposition::Excluded { .. }) {
            return Err(ParityValidationError::UpdaterExclusionForbidden);
        }
        Ok(Self {
            id,
            taxonomy,
            title,
            disposition,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceAlgorithm {
    Sha256,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvidenceChecksum {
    pub algorithm: EvidenceAlgorithm,
    #[serde(rename = "lowercaseHex")]
    pub lowercase_hex: String,
}
impl EvidenceChecksum {
    pub fn new(
        algorithm: EvidenceAlgorithm,
        lowercase_hex: impl Into<String>,
    ) -> Result<Self, ParityValidationError> {
        let lowercase_hex = lowercase_hex.into();
        let bytes = lowercase_hex.as_bytes();
        if bytes.len() != 64
            || !bytes
                .iter()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
        {
            return Err(ParityValidationError::InvalidChecksum);
        }
        Ok(Self {
            algorithm,
            lowercase_hex,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvidenceArtifact {
    pub id: ParityStableID,
    pub locator: String,
    pub checksum: EvidenceChecksum,
    #[serde(rename = "capturedAtMilliseconds")]
    pub captured_at_milliseconds: u64,
}
impl EvidenceArtifact {
    pub fn new(
        id: ParityStableID,
        locator: impl Into<String>,
        checksum: EvidenceChecksum,
        captured_at_milliseconds: u64,
    ) -> Result<Self, ParityValidationError> {
        let locator = locator.into();
        if locator.is_empty() {
            return Err(ParityValidationError::InvalidStableID);
        }
        Ok(Self {
            id,
            locator,
            checksum,
            captured_at_milliseconds,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvidenceBinding {
    #[serde(rename = "manifestID")]
    pub manifest_id: ParityStableID,
    #[serde(rename = "artifactID")]
    pub artifact_id: ParityStableID,
    pub assertion: String,
}
impl EvidenceBinding {
    pub fn new(
        manifest_id: ParityStableID,
        artifact_id: ParityStableID,
        assertion: impl Into<String>,
    ) -> Result<Self, ParityValidationError> {
        let assertion = assertion.into();
        if assertion.is_empty() {
            return Err(ParityValidationError::InvalidStableID);
        }
        Ok(Self {
            manifest_id,
            artifact_id,
            assertion,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FreshnessPolicy {
    #[serde(rename = "maximumAgeMilliseconds")]
    pub maximum_age_milliseconds: u64,
}
impl FreshnessPolicy {
    pub fn new(maximum_age_milliseconds: u64) -> Result<Self, ParityValidationError> {
        if maximum_age_milliseconds == 0 {
            return Err(ParityValidationError::StaleEvidence("policy".into()));
        }
        Ok(Self {
            maximum_age_milliseconds,
        })
    }
    pub fn validate(
        &self,
        artifact: &EvidenceArtifact,
        now_milliseconds: u64,
    ) -> Result<(), ParityValidationError> {
        if artifact.captured_at_milliseconds > now_milliseconds
            || now_milliseconds - artifact.captured_at_milliseconds
                > self.maximum_age_milliseconds
        {
            return Err(ParityValidationError::StaleEvidence(
                artifact.id.raw_value.clone(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CoverageStatus {
    Provisional { note: String },
    Verified { evidence: HashSet<ParityStableID> },
    Excluded(ExclusionReason),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageRecord {
    #[serde(rename = "manifestID")]
    pub manifest_id: ParityStableID,
    pub status: CoverageStatus,
}

pub struct CoverageValidator;
impl CoverageValidator {
    pub fn validate_manifest(items: &[ManifestItem]) -> Result<(), ParityValidationError> {
        let mut ids: HashSet<&ParityStableID> = HashSet::new();
        for item in items {
            if !ids.insert(&item.id) {
                return Err(ParityValidationError::DuplicateID(item.id.raw_value.clone()));
            }
        }
        Ok(())
    }

    pub fn validate_provisional_coverage(
        items: &[ManifestItem],
        coverage: &[CoverageRecord],
    ) -> Result<(), ParityValidationError> {
        Self::validate_manifest(items)?;
        let items_by_id: HashMap<&ParityStableID, &ManifestItem> =
            items.iter().map(|i| (&i.id, i)).collect();
        let mut covered: HashSet<&ParityStableID> = HashSet::new();
        for record in coverage {
            if !covered.insert(&record.manifest_id) {
                return Err(ParityValidationError::DuplicateID(
                    record.manifest_id.raw_value.clone(),
                ));
            }
            let Some(item) = items_by_id.get(&record.manifest_id) else {
                return Err(ParityValidationError::MissingEvidence(
                    record.manifest_id.raw_value.clone(),
                ));
            };
            let ok = match (&item.disposition, &record.status) {
                (ParityDisposition::Required, CoverageStatus::Provisional { note }) => {
                    !note.is_empty()
                }
                (ParityDisposition::Required, CoverageStatus::Verified { evidence }) => {
                    !evidence.is_empty()
                }
                (
                    ParityDisposition::Excluded { reason: expected },
                    CoverageStatus::Excluded(actual),
                ) => expected == actual,
                _ => false,
            };
            if !ok {
                return Err(ParityValidationError::MissingEvidence(
                    record.manifest_id.raw_value.clone(),
                ));
            }
        }
        Ok(())
    }

    pub fn validate_terminal_coverage(
        items: &[ManifestItem],
        coverage: &[CoverageRecord],
        artifacts: &[EvidenceArtifact],
        bindings: &[EvidenceBinding],
        freshness: &FreshnessPolicy,
        now_milliseconds: u64,
    ) -> Result<(), ParityValidationError> {
        Self::validate_manifest(items)?;
        let mut artifact_by_id: HashMap<&ParityStableID, &EvidenceArtifact> = HashMap::new();
        for artifact in artifacts {
            if artifact_by_id.insert(&artifact.id, artifact).is_some() {
                return Err(ParityValidationError::DuplicateID(artifact.id.raw_value.clone()));
            }
        }
        let mut coverage_by_id: HashMap<&ParityStableID, &CoverageStatus> = HashMap::new();
        for record in coverage {
            if coverage_by_id
                .insert(&record.manifest_id, &record.status)
                .is_some()
            {
                return Err(ParityValidationError::DuplicateID(
                    record.manifest_id.raw_value.clone(),
                ));
            }
        }
        let mut bindings_by_item: HashMap<&ParityStableID, HashSet<&ParityStableID>> =
            HashMap::new();
        for binding in bindings {
            bindings_by_item
                .entry(&binding.manifest_id)
                .or_default()
                .insert(&binding.artifact_id);
        }
        for item in items {
            let Some(status) = coverage_by_id.get(&item.id) else {
                return Err(ParityValidationError::MissingEvidence(
                    item.id.raw_value.clone(),
                ));
            };
            match (&item.disposition, status) {
                (
                    ParityDisposition::Excluded { reason: expected },
                    CoverageStatus::Excluded(actual),
                ) if expected == actual => continue,
                (ParityDisposition::Required, CoverageStatus::Verified { evidence }) => {
                    if evidence.is_empty() {
                        return Err(ParityValidationError::MissingEvidence(
                            item.id.raw_value.clone(),
                        ));
                    }
                    let empty = HashSet::new();
                    let bound_ids = bindings_by_item.get(&item.id).unwrap_or(&empty);
                    if !evidence
                        .iter()
                        .all(|e| bound_ids.iter().any(|b| *b == e))
                    {
                        return Err(ParityValidationError::MissingEvidence(
                            item.id.raw_value.clone(),
                        ));
                    }
                    for evidence_id in evidence {
                        let Some(artifact) = artifact_by_id.get(&evidence_id) else {
                            return Err(ParityValidationError::MissingEvidence(
                                evidence_id.raw_value.clone(),
                            ));
                        };
                        freshness.validate(artifact, now_milliseconds)?;
                    }
                }
                (_, CoverageStatus::Provisional { .. }) => {
                    return Err(ParityValidationError::ProvisionalCoverage(
                        item.id.raw_value.clone(),
                    ));
                }
                _ => {
                    return Err(ParityValidationError::MissingEvidence(
                        item.id.raw_value.clone(),
                    ));
                }
            }
        }
        let item_ids: HashSet<&ParityStableID> = items.iter().map(|i| &i.id).collect();
        let covered_ids: HashSet<&ParityStableID> = coverage_by_id.keys().copied().collect();
        if covered_ids != item_ids {
            return Err(ParityValidationError::MissingEvidence(
                "unbound-coverage".into(),
            ));
        }
        Ok(())
    }
}
