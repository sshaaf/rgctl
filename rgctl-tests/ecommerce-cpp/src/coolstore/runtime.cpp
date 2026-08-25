#include "ecommerce/coolstore/runtime.hpp"

namespace ecommerce::coolstore {

CoolstoreRuntime& runtime() {
    static CoolstoreRuntime instance;
    return instance;
}

}  // namespace ecommerce::coolstore
