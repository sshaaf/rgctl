#ifndef EC_SQLITE_H
#define EC_SQLITE_H
#include <sqlite3.h>
int db_open(const char *path, sqlite3 **db);
int db_close(sqlite3 *db);
int db_exec(sqlite3 *db, const char *sql);
#endif
