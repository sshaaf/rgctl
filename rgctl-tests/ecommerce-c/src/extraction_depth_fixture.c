/* Extraction-depth probe: fn pointers, local include, typedef struct alias. */

#include "ecommerce/extraction_depth_local.h"
#include <stdio.h>

typedef struct Cart Cart;

typedef void (*handler_fn)(void);

void extraction_depth_dispatch(handler_fn handler) {
    handler();
    (*handler)();
}

int extraction_depth_init(void) {
    return 0;
}
