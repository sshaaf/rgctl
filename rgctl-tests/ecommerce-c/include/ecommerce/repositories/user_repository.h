#ifndef EC_USER_REPO_H
#define EC_USER_REPO_H
#include <sqlite3.h>
#include "ecommerce/types.h"
int user_repo_find_by_id(sqlite3 *db, int id, void *out);
int user_repo_find_by_email(sqlite3 *db, int id, void *out);
int user_repo_create(sqlite3 *db, const void *entity);
int user_repo_exists_by_email(sqlite3 *db, const char *email);
#endif
