#include "ecommerce/coolstore/services/coolstore_product_service.hpp"

namespace ecommerce::coolstore {

CoolstoreProductService::CoolstoreProductService() {
    seed("329299", "Red Fedora", "Official Red Hat Fedora", 34.99);
    seed("329199", "Forge Laptop Sticker", "JBoss Community sticker", 8.50);
    seed("165613", "Solid Performance Polo", "Moisture-wicking polo", 17.80);
    seed("165614", "Ogios T-shirt", "CoolStore tee", 11.50);
    seed("165954", "Quarkus Stickers", "Pack of stickers", 9.99);
}

void CoolstoreProductService::seed(const std::string& id, const std::string& name,
                                   const std::string& desc, double price) {
    catalog_.emplace(id, CatalogProduct(id, name, desc, price));
}

std::vector<CatalogProduct> CoolstoreProductService::getProducts() const {
    std::vector<CatalogProduct> out;
    out.reserve(catalog_.size());
    for (const auto& [_, p] : catalog_) {
        out.push_back(p);
    }
    return out;
}

CatalogProduct* CoolstoreProductService::getProductByItemId(const std::string& itemId) {
    auto it = catalog_.find(itemId);
    if (it == catalog_.end()) return nullptr;
    return &it->second;
}

}  // namespace ecommerce::coolstore
