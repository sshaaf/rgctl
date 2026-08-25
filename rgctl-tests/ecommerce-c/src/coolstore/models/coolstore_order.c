#include "ecommerce/coolstore/models/coolstore_order.h"
#include <string.h>

void coolstore_order_init(coolstore_order_t *order) {
    if (order) {
        memset(order, 0, sizeof(*order));
    }
}
