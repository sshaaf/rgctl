#ifndef EC_PRODUCT_HANDLER_H
#define EC_PRODUCT_HANDLER_H
#include <sqlite3.h>
int handle_product(sqlite3 *db, const char *query);
#endif
