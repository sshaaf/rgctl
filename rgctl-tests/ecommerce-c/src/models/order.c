#include "ecommerce/models/order.h"
#include <string.h>

void order_init(order_t *obj) { if (obj) { memset(obj, 0, sizeof(*obj)); } }

void order_add_total(order_t *order, double amount) { if (order) order->total += amount; }

int order_is_paid(const order_t *order) { return order && order->status == 2; }

