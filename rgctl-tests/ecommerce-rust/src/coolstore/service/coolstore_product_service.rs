use std::collections::HashMap;
use std::sync::RwLock;

use crate::coolstore::model::CatalogProduct;

pub struct CoolstoreProductService {
    catalog: RwLock<HashMap<String, CatalogProduct>>,
}

impl CoolstoreProductService {
    pub fn new() -> Self {
        let svc = Self {
            catalog: RwLock::new(HashMap::new()),
        };
        svc.seed("329299", "Red Fedora", "Official Red Hat Fedora", 34.99);
        svc.seed("329199", "Forge Laptop Sticker", "JBoss Community sticker", 8.50);
        svc.seed("165613", "Solid Performance Polo", "Moisture-wicking polo", 17.80);
        svc.seed("165614", "Ogios T-shirt", "CoolStore tee", 11.50);
        svc.seed("165954", "Quarkus Stickers", "Pack of stickers", 9.99);
        svc
    }

    fn seed(&self, id: &str, name: &str, desc: &str, price: f64) {
        self.catalog
            .write()
            .expect("catalog lock")
            .insert(id.to_string(), CatalogProduct::new(id, name, desc, price));
    }

    pub fn get_products(&self) -> Vec<CatalogProduct> {
        self.catalog
            .read()
            .expect("catalog lock")
            .values()
            .cloned()
            .collect()
    }

    pub fn get_product_by_item_id(&self, item_id: &str) -> Option<CatalogProduct> {
        self.catalog.read().expect("catalog lock").get(item_id).cloned()
    }
}

impl Default for CoolstoreProductService {
    fn default() -> Self {
        Self::new()
    }
}
