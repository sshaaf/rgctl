#pragma once
#include "ecommerce/coolstore/models/catalog_product.hpp"
#include <string>
#include <unordered_map>
#include <vector>

namespace ecommerce::coolstore {

class CoolstoreProductService {
public:
    CoolstoreProductService();
    std::vector<CatalogProduct> getProducts() const;
    CatalogProduct* getProductByItemId(const std::string& itemId);

private:
    void seed(const std::string& id, const std::string& name, const std::string& desc, double price);
    std::unordered_map<std::string, CatalogProduct> catalog_;
};

}  // namespace ecommerce::coolstore
