use anyhow::Result;
use async_trait::async_trait;
use fluent_pipeline::Mergeable;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use crate::prelude::{TransferData, TransferDataValue};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DefaultMerge;

#[async_trait]
impl Mergeable<Arc<TransferData>> for DefaultMerge {
    async fn merge(&self, input: &[Option<&Arc<TransferData>>]) -> Result<Arc<TransferData>> {
        let mut merge = Vec::new();
        for data in input.iter().flatten() {
            merge.push(Arc::clone(data));
        }
        Ok(Arc::new(TransferData {
            previous: None,
            value: TransferDataValue::Merge(merge),
        }))
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum Transform {
    #[default]
    IncludeHistory,
}
