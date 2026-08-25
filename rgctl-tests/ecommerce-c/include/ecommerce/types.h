#ifndef ECOMMERCE_TYPES_H
#define ECOMMERCE_TYPES_H

#include <stddef.h>

typedef struct {
    int id;
    char email[128];
    char password_hash[64];
} user_t;

typedef struct {
    int id;
    char name[128];
    double price;
    int category_id;
} product_t;

typedef struct {
    int id;
    int user_id;
    int product_id;
    int quantity;
} cart_item_t;

typedef struct {
    int id;
    int user_id;
    double total;
    int status;
} order_t;

typedef struct {
    int id;
    int product_id;
    int quantity;
} inventory_t;

typedef struct {
    int id;
    char name[64];
} category_t;

#endif
