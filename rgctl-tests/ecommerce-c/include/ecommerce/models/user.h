#ifndef EC_USER_H
#define EC_USER_H
#include "ecommerce/types.h"
void user_init(user_t *obj);
void user_set_email(user_t *u, const char *email);
int user_validate_email(const user_t *obj);
#endif
