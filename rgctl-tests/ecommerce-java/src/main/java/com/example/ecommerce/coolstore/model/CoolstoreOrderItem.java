package com.example.ecommerce.coolstore.model;

public class CoolstoreOrderItem {
    private String productId;
    private int quantity;
    private double price;

    public CoolstoreOrderItem() {}

    public CoolstoreOrderItem(String productId, int quantity, double price) {
        this.productId = productId;
        this.quantity = quantity;
        this.price = price;
    }

    public String getProductId() { return productId; }
    public void setProductId(String productId) { this.productId = productId; }
    public int getQuantity() { return quantity; }
    public void setQuantity(int quantity) { this.quantity = quantity; }
    public double getPrice() { return price; }
    public void setPrice(double price) { this.price = price; }
}
