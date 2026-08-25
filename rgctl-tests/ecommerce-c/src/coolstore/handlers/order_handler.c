#include "ecommerce/coolstore/handlers/order_handler.h"
#include "ecommerce/coolstore/services/coolstore_order_service.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const char *skip_method(const char *query) {
    const char *p = query;
    if (!p) return NULL;
    if (strncmp(p, "GET ", 4) == 0) return p + 4;
    if (strncmp(p, "POST ", 5) == 0) return p + 5;
    if (strncmp(p, "DELETE ", 7) == 0) return p + 7;
    return p;
}

int handle_coolstore_orders(const char *query) {
    const char *path;
    coolstore_order_t orders[COOLSTORE_MAX_ORDERS];
    int count = 0;
    long orderId = 0;

    if (!query) return -1;
    path = skip_method(query);
    if (!path || !strstr(path, "/services/orders")) return 0;

    /* GET /services/orders/{orderId} */
    if (sscanf(path, "/services/orders/%ld", &orderId) == 1) {
        return coolstore_order_get_by_id(orderId) ? 0 : -1;
    }

    /* GET /services/orders */
    if (strcmp(path, "/services/orders") == 0 || strstr(query, "/services/orders") != NULL) {
        return coolstore_order_get_orders(orders, COOLSTORE_MAX_ORDERS, &count);
    }
    return 0;
}
