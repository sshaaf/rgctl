package com.example.ecommerce.coolstore.service;

import com.example.ecommerce.coolstore.model.CoolstoreOrder;
import com.example.ecommerce.coolstore.model.CoolstoreOrderItem;
import com.example.ecommerce.coolstore.model.ShoppingCart;
import com.example.ecommerce.coolstore.model.ShoppingCartItem;
import org.springframework.stereotype.Service;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicLong;

@Service
public class CoolstoreOrderService {

    private final AtomicLong seq = new AtomicLong(1);
    private final Map<Long, CoolstoreOrder> orders = new ConcurrentHashMap<>();

    public CoolstoreOrder process(ShoppingCart cart) {
        CoolstoreOrder order = new CoolstoreOrder();
        order.setOrderId(seq.getAndIncrement());
        order.setCartId(cart.getCartId());
        order.setCartTotal(cart.getCartTotal());
        List<CoolstoreOrderItem> items = new ArrayList<>();
        for (ShoppingCartItem sci : cart.getShoppingCartItemList()) {
            if (sci.getProduct() != null) {
                items.add(new CoolstoreOrderItem(
                        sci.getProduct().getItemId(),
                        sci.getQuantity(),
                        sci.getPrice()));
            }
        }
        order.setItems(items);
        orders.put(order.getOrderId(), order);
        return order;
    }

    public List<CoolstoreOrder> getOrders() {
        return List.copyOf(orders.values());
    }

    public Optional<CoolstoreOrder> getOrderById(long orderId) {
        return Optional.ofNullable(orders.get(orderId));
    }
}
