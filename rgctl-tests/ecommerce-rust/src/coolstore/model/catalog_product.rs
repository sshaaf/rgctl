use serde::{Deserialize, Serialize};

/// Lightweight CoolStore catalog product (itemId keyed).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogProduct {
    pub item_id: String,
    pub name: String,
    pub desc: String,
    pub price: f64,
}

impl CatalogProduct {
    pub fn new(item_id: impl Into<String>, name: impl Into<String>, desc: impl Into<String>, price: f64) -> Self {
        Self {
            item_id: item_id.into(),
            name: name.into(),
            desc: desc.into(),
            price,
        }
    }
}
