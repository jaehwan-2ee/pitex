## Summary

<!-- What changed and why. Link the issue: Fixes #123 -->

## Platform

- [ ] macOS (`Mac/`, `Packages/`)
- [ ] Linux (`Linux/`)
- [ ] Tooling / gates (`Tools/`)

## Verification

- [ ] `node Tools/verify-target-dag.mjs` passes
- [ ] `node Tools/verify-rust-dag.mjs` passes
- [ ] `node Tools/validate-xcodeproj.mjs` passes
- [ ] `node Tools/verify-release-surface.mjs` passes
- [ ] `swift test` passes in `Packages/TexCore` and `Packages/TexApp`
- [ ] `cargo test --workspace` passes in `Linux/` (or reason noted)
