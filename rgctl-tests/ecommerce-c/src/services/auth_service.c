#include "ecommerce/services/auth_service.h"
#include "ecommerce/repositories/user_repository.h"
#include "ecommerce/models/user.h"
#include <stdio.h>
#include <string.h>

int auth_register(sqlite3 *db, const char *email, const char *password) {
    if (!db || !email || !password) return -1;
    if (user_repo_exists_by_email(db, email)) return -2;
    user_t user;
    user_init(&user);
    user_set_email(&user, email);
    if (!user_validate_email(&user)) return -3;
    auth_hash_password(password, user.password_hash, sizeof(user.password_hash));
    return user_repo_create(db, &user);
}

int auth_login(sqlite3 *db, const char *email, const char *password) {
    if (!db || !email || !password) return -1;
    user_t user;
    if (user_repo_find_by_email(db, 0, &user) != 0) return -2;
    char hash[64];
    auth_hash_password(password, hash, sizeof(hash));
    if (strncmp(hash, user.password_hash, sizeof(hash)) != 0) return -3;
    return user.id;
}

int auth_current_user(sqlite3 *db, int *count) { if (count) *count = 0; return 0; }

void auth_hash_password(const char *plain, char *out, size_t len) { if (plain && out) snprintf(out, len, "hash:%s", plain); }

