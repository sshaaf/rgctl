#ifndef EC_COOLSTORE_ORDER_HANDLER_H
#define EC_COOLSTORE_ORDER_HANDLER_H

/** Dispatch GET /services/orders and GET /services/orders/{orderId}. */
int handle_coolstore_orders(const char *query);

#endif
