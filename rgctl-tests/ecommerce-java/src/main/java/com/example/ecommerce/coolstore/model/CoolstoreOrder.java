package com.example.ecommerce.coolstore.model;

import java.util.ArrayList;
import java.util.List;

public class CoolstoreOrder {
    private long orderId;
    private String cartId;
    private double cartTotal;
    private List<CoolstoreOrderItem> items = new ArrayList<>();

    public CoolstoreOrder() {}

    public long getOrderId() { return orderId; }
    public void setOrderId(long orderId) { this.orderId = orderId; }
    public String getCartId() { return cartId; }
    public void setCartId(String cartId) { this.cartId = cartId; }
    public double getCartTotal() { return cartTotal; }
    public void setCartTotal(double cartTotal) { this.cartTotal = cartTotal; }
    public List<CoolstoreOrderItem> getItems() { return items; }
    public void setItems(List<CoolstoreOrderItem> items) { this.items = items; }
}
