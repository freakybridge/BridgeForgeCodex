//! Shared test-only release plan application; never part of the product runtime.
use super::{
    FileReleasePlan, verify_release_inputs, verify_release_inventory, verify_release_status,
};

pub(crate) fn apply_file_release_plan(plan: &FileReleasePlan) -> Result<(), String> {
    let root = plan
        .writes
        .keys()
        .find(|p| p.file_name().is_some_and(|n| n == "VERSION"))
        .and_then(|p| p.parent())
        .ok_or("release root missing")?;
    plan.build_directories.verify(root)?;
    if let Some(environment) = &plan.environment {
        let root = plan
            .writes
            .keys()
            .find(|p| p.file_name().is_some_and(|n| n == "VERSION"))
            .and_then(|p| p.parent())
            .ok_or("release root missing")?;
        environment.verify(root)?;
    }
    verify_release_inputs(&plan.inputs)?;
    if let Some(inventory) = &plan.release_inventory {
        let root = plan
            .writes
            .keys()
            .find(|p| p.file_name().is_some_and(|n| n == "VERSION"))
            .and_then(|p| p.parent())
            .ok_or("release root missing")?;
        verify_release_inventory(root, &bridgeforge_core::SystemProcessRunner, inventory)?;
        if let Some(status) = &plan.release_status {
            verify_release_status(root, &bridgeforge_core::SystemProcessRunner, status)?;
        }
    }
    for (path, payload) in &plan.writes {
        bridgeforge_core::persistence::atomic_write(path, payload)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}
