#ifndef EC_ORDER_HANDLER_H
#define EC_ORDER_HANDLER_H
#include <sqlite3.h>
int handle_order(sqlite3 *db, const char *query);
#endif
