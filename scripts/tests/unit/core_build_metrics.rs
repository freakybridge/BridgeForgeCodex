use super::*;
use std::{
    cell::Cell,
    path::{Path, PathBuf},
};

#[test]
fn factory_adapter_preserves_the_callers_process_observer() {
    struct Adapter(Cell<usize>);
    impl ProcessRunner for Adapter {
        fn run(&self, _request: &ProcessRequest) -> std::io::Result<ProcessOutput> {
            self.0.set(self.0.get() + 1);
            Ok(ProcessOutput {
                code: 0,
                stdout: vec![],
                stderr: vec![],
                timed_out: false,
            })
        }
        fn factory_transaction(
            &self,
            root: &Path,
            writes: BTreeMap<PathBuf, Vec<u8>>,
            _inputs: BTreeMap<PathBuf, Option<Vec<u8>>>,
            executor: &dyn ProcessRunner,
        ) -> Result<crate::factory_bridge::FactoryTransaction, String> {
            let mut request = ProcessRequest::new("cargo", root);
            request.args = vec!["build".into()];
            executor.run(&request).map_err(|e| e.to_string())?;
            Ok(crate::factory_bridge::FactoryTransaction {
                generated_reused: 0,
                generated_built: 1,
                writes,
                binaries: Default::default(),
                before: Default::default(),
                verify: Box::new(|_| Ok(())),
            })
        }
    }
    let adapter = Adapter(Cell::new(0));
    let observed = Observed {
        runner: &adapter,
        metrics: Default::default(),
    };
    observed
        .factory_transaction(Path::new("."), BTreeMap::new(), BTreeMap::new(), &observed)
        .unwrap();
    assert_eq!(adapter.0.get(), 1);
    assert_eq!(observed.metrics.borrow().cargo_build_invocations, 1);
}
