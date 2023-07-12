use anyhow::Result;
use turbo_tasks::{emit, primitives::StringVc};

/// An arbitrary metadata payload can be used to analyze, diagnose
/// Turbopack's behavior.
#[turbo_tasks::value_trait]
pub trait Diagnostics {
    // [TODO]: These are subject to change; not finalized yet.
    fn category(&self) -> StringVc;
    fn key(&self) -> StringVc;
    fn value(&self) -> StringVc;
}

impl DiagnosticsVc {
    pub fn emit(self) {
        emit(self);
    }

    pub async fn peek_diagnostics_with_path<T: turbo_tasks::CollectiblesSource + Copy>(
        source: T,
    ) -> Result<CapturedDiagnosticsVc> {
        Ok(CapturedDiagnosticsVc::cell(CapturedDiagnostics {
            diagnostics: source.peek_collectibles().strongly_consistent().await?,
        }))
    }
}

/// A list of diagnostics captured with
/// [`DiagnosticsVc::peek_diagnostics_with_path`] and
#[derive(Debug)]
#[turbo_tasks::value]
pub struct CapturedDiagnostics {
    pub diagnostics: auto_hash_map::AutoSet<DiagnosticsVc>,
}
