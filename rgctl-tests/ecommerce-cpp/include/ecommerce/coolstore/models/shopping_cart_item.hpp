#pragma once
#include "ecommerce/coolstore/models/catalog_product.hpp"

namespace ecommerce::coolstore {

class ShoppingCartItem {
public:
    double price{};
    int quantity{};
    double promoSavings{};
    CatalogProduct product;
    bool hasProduct{false};
};

}  // namespace ecommerce::coolstore
