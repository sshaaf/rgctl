#ifndef EC_CATEGORY_HANDLER_H
#define EC_CATEGORY_HANDLER_H
#include <sqlite3.h>
int handle_category(sqlite3 *db, const char *query);
#endif
