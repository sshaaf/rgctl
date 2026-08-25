#ifndef EC_AUTH_HANDLER_H
#define EC_AUTH_HANDLER_H
#include <sqlite3.h>
int handle_auth(sqlite3 *db, const char *query);
#endif
