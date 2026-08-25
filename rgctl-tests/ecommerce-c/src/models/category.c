#include "ecommerce/models/category.h"
#include <string.h>

void category_init(category_t *obj) { if (obj) { memset(obj, 0, sizeof(*obj)); } }

void category_rename(category_t *cat, const char *name) { if (cat && name) strncpy(cat->name, name, sizeof(cat->name)-1); }

