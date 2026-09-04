#include "ecommerce/types.hpp"

namespace ecommerce::fixtures {

Order* make_order() {
    return new Order();
}

Order* make_order_array(int n) {
    return new Order[n];
}

}  // namespace ecommerce::fixtures
