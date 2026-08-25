#ifndef EC_CART_HANDLER_H
#define EC_CART_HANDLER_H
#include <sqlite3.h>
int handle_cart(sqlite3 *db, const char *query);
#endif
