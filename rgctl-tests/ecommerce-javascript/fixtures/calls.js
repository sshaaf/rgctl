class OrderService {
  checkout() {
    return new OrderDto();
  }

  dynamicCall(obj, key) {
    return obj[key]();
  }
}

class OrderDto {}
