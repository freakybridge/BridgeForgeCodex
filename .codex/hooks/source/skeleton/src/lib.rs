pub use bridgeforge_core::build_manifest as manifest;
pub use bridgeforge_core::{BaselineReport, BaselineState};
pub use bridgeforge_core::{
    CommandOutcome, EXIT_BLOCKED, EXIT_OK, ProcessOutput, ProcessRequest, ProcessRunner,
    ProjectContext, SystemProcessRunner,
};
pub use bridgeforge_core::{
    artifact_cache, build_inputs, build_metrics, file_lock, generated_assets, json,
    managed_markdown, managed_paths, persistence, release_metrics, runtime, source_projection,
    user_agents,
};
pub mod baseline;
pub mod content;
pub mod project_hooks;
pub fn migration_enabled(contract: &serde_json::Value) -> bool {
    contract["assets"].as_array().is_some_and(|a| {
        a.iter().any(|a| {
            a["id"] == "codex.project-hook-policy"
                && a["target"] == ".codex/project-hook-policy.json"
        })
    })
}
