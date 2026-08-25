#include "ecommerce/models/user.h"
#include <string.h>

void user_init(user_t *obj) { if (obj) { memset(obj, 0, sizeof(*obj)); } }

void user_set_email(user_t *u, const char *email) { if (u && email) strncpy(u->email, email, sizeof(u->email)-1); }

int user_validate_email(const user_t *u) { return u && u->email[0] != '\0' && strchr(u->email, '@') != NULL; }

