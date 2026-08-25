#ifndef EC_AUTH_SERVICE_H
#define EC_AUTH_SERVICE_H
#include <sqlite3.h>
#include "ecommerce/types.h"
int auth_register(sqlite3 *db, const char *a, const char *b);
int auth_login(sqlite3 *db, const char *a, const char *b);
int auth_current_user(sqlite3 *db, int id, void *out);
void auth_hash_password(const char *plain, char *out, size_t len);
#endif
