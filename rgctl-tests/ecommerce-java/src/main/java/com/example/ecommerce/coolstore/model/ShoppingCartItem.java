package com.example.ecommerce.coolstore.model;

public class ShoppingCartItem {
    private double price;
    private int quantity;
    private double promoSavings;
    private CatalogProduct product;

    public ShoppingCartItem() {}

    public double getPrice() { return price; }
    public void setPrice(double price) { this.price = price; }

    public int getQuantity() { return quantity; }
    public void setQuantity(int quantity) { this.quantity = quantity; }

    public double getPromoSavings() { return promoSavings; }
    public void setPromoSavings(double promoSavings) { this.promoSavings = promoSavings; }

    public CatalogProduct getProduct() { return product; }
    public void setProduct(CatalogProduct product) { this.product = product; }
}
