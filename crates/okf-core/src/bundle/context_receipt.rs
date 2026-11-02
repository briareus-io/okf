//! Stateless freshness receipts. These are neither credentials nor evidence of comprehension.
use super::manifest::{self, BundleContext, ContextRequest};
use crate::error::{OkfError, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextReceipt {
    pub version: u32,
    pub root: PathBuf,
    pub selection: ContextRequest,
    pub state_components: BTreeMap<String, String>,
}

fn invalid(message: impl std::fmt::Display) -> OkfError {
    OkfError::Usage(format!("context-invalid: {message}; reload bundle context"))
}

impl ContextReceipt {
    pub fn issue(context: &BundleContext) -> Result<String> {
        if context.selection.inspection || context.selection.all {
            return Err(invalid(
                "inspection context cannot issue an execution token",
            ));
        }
        if context
            .records
            .iter()
            .any(|r| r["inputs_complete"] == false)
        {
            return Err(invalid("selected workflow inputs are incomplete"));
        }
        serde_json::to_string(&Self {
            version: 1,
            root: context.root.clone(),
            selection: context.selection.clone(),
            state_components: context.state_components.clone(),
        })
        .map_err(invalid)
    }

    pub fn parse(token: &str) -> Result<Self> {
        if token.len() > 1024 * 1024 {
            return Err(invalid("token exceeds 1 MiB"));
        }
        let receipt: Self = serde_json::from_str(token).map_err(invalid)?;
        if receipt.version != 1
            || receipt.selection.inspection
            || receipt.selection.all
            || receipt.selection.revision.is_some()
        {
            return Err(invalid("unsupported token version or inspection selection"));
        }
        Ok(receipt)
    }

    /// Rebuild precisely the original selection, then check requested operation coverage.
    /// A different operation subset must not cause false digest mismatches.
    pub fn check(
        &self,
        root: &Path,
        operations: &[String],
        commands: &[String],
    ) -> Result<BundleContext> {
        if std::fs::canonicalize(root)? != self.root {
            return Err(invalid("bundle root differs"));
        }
        for operation in operations {
            if !self.selection.operations.contains(operation) {
                return Err(invalid(format!("operation {operation:?} was not covered")));
            }
        }
        let current = manifest::context(root, self.selection.clone(), commands)?;
        if current.selection.inspection {
            return Err(invalid(
                "execution token cannot be used for historical inspection",
            ));
        }
        let changed: Vec<_> = self
            .state_components
            .keys()
            .chain(current.state_components.keys())
            .filter(|key| self.state_components.get(*key) != current.state_components.get(*key))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        if !changed.is_empty() {
            return Err(invalid(format!(
                "changed dependencies: {}",
                changed.join(", ")
            )));
        }
        if current
            .records
            .iter()
            .any(|r| r["inputs_complete"] == false)
        {
            return Err(invalid("selected workflow inputs are incomplete"));
        }
        Ok(current)
    }
}
