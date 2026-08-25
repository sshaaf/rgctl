#pragma once
#include "ecommerce/coolstore/models/shopping_cart.hpp"

namespace ecommerce::coolstore {

class ShippingService {
public:
    double calculateShipping(const ShoppingCart& sc) const;
    double calculateShippingInsurance(const ShoppingCart& sc) const;

private:
    static double round2(double v);
};

}  // namespace ecommerce::coolstore
