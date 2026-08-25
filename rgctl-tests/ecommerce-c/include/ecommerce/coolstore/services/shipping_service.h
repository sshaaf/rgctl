#ifndef EC_COOLSTORE_SHIPPING_SERVICE_H
#define EC_COOLSTORE_SHIPPING_SERVICE_H

#include "ecommerce/coolstore/models/shopping_cart.h"

double shipping_calculate(const shopping_cart_t *sc);
double shipping_calculate_insurance(const shopping_cart_t *sc);

#endif
