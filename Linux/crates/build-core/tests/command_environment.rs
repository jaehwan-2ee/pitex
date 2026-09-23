use build_core::*;

#[test]
fn environment_keys_follow_native_rules() {
    for (key, valid) in [("PATH", true), ("=C:", cfg!(windows)), ("BAD=KEY", false), ("=C:=BAD", false), ("", false), ("BAD\0KEY", false)] {
        let policy = EnvironmentPolicy::Replace([(key.into(), "value".into())].into_iter().collect());
        assert_eq!(policy.validated().is_ok(), valid, "{key:?}");
    }
}

#[cfg(windows)]
#[test]
fn windows_batch_preserves_paths_arguments_and_drive_environment() {
    let root = std::env::temp_dir().join(format!("pitex runner's & test {}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let script = root.join("test command.cmd");
    std::fs::write(&script, "@echo off\r\nif \"%~1\"==\"space's & value\" (echo ok) else (exit /b 3)\r\n").unwrap();
    let plan = DirectCommandPlan::new(
        script.to_string_lossy(), vec!["space's & value".into()],
        WorkingDirectoryPolicy::ProjectRoot,
        EnvironmentPolicy::Inherit { overrides: [("=C:".into(), "C:\\".into())].into_iter().collect() },
    ).unwrap();
    let result = ProcessRunner::default().run(&plan, &root, None, Some(std::time::Duration::from_secs(10)), None, None).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(result.termination, ProcessTermination::Exited { code: 0 }, "{}", String::from_utf8_lossy(&result.standard_error));
    assert_eq!(String::from_utf8_lossy(&result.standard_output).trim(), "ok");
}
