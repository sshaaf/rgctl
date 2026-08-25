#ifndef EC_HEALTH_HANDLER_H
#define EC_HEALTH_HANDLER_H
#include <sqlite3.h>
int handle_health(sqlite3 *db, const char *query);
#endif
