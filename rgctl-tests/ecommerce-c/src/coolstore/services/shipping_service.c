#include "ecommerce/coolstore/services/shipping_service.h"

static double round2(double v) {
    return (double)((long)(v * 100.0 + 0.5)) / 100.0;
}

double shipping_calculate(const shopping_cart_t *sc) {
    double total;
    if (!sc) return 0;
    total = sc->cartItemTotal;
    if (total >= 0 && total < 25) return 2.99;
    if (total >= 25 && total < 50) return 4.99;
    if (total >= 50 && total < 75) return 6.99;
    if (total >= 75 && total < 100) return 8.99;
    if (total >= 100) return 10.99;
    return 0;
}

double shipping_calculate_insurance(const shopping_cart_t *sc) {
    double total;
    if (!sc) return 0;
    total = sc->cartItemTotal;
    if (total >= 25 && total < 100) return round2(total * 0.02);
    if (total >= 100) return round2(total * 0.015);
    return 0;
}
