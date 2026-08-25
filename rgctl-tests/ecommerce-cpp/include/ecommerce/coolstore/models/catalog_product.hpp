#pragma once
#include <string>

namespace ecommerce::coolstore {

class CatalogProduct {
public:
    CatalogProduct() = default;
    CatalogProduct(std::string itemId, std::string name, std::string desc, double price)
        : itemId(std::move(itemId)), name(std::move(name)), desc(std::move(desc)), price(price) {}

    std::string itemId;
    std::string name;
    std::string desc;
    double price{};
};

}  // namespace ecommerce::coolstore
