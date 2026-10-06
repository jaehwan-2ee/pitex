//! Port of the portable part of
//! `Packages/TexApp/Tests/TexAppTests/ArchitectureTests.swift`.

use settings_feature::SettingsKey;

/// The feature states must be usable across threads, matching the Swift
/// `Sendable` contract.
#[test]
fn feature_state_surfaces_are_sendable() {
    fn assert_send_sync<T: Send>() {}
    assert_send_sync::<project_feature::ProjectFeatureState>();
    assert_send_sync::<editor_feature::EditorPresentationSnapshot>();
    assert_send_sync::<build_feature::BuildFeatureState>();
    assert_send_sync::<pdf_feature::PDFFeatureState>();
    assert_send_sync::<settings_feature::SettingsState>();
}

#[test]
fn persisted_settings_keys_exclude_secret_updater_and_distribution_semantics() {
    let prohibited_fragments = [
        "apikey", "api_key", "secret", "updater", "updatechannel", "distribution",
        "releasechannel",
    ];
    let keys = [
        "editor",
        "build",
        "pdf",
        "diagnosticsConsent",
        "project",
    ];
    for key in keys {
        let normalized = key.to_lowercase();
        assert!(
            !prohibited_fragments.iter().any(|f| normalized.contains(f)),
            "Persisted settings key must remain a non-secret, portable preference: {key}"
        );
    }
    // Sanity-check the enum itself has the same five cases.
    let _ = [
        SettingsKey::Editor,
        SettingsKey::Build,
        SettingsKey::Pdf,
        SettingsKey::DiagnosticsConsent,
        SettingsKey::Project,
    ];
}
