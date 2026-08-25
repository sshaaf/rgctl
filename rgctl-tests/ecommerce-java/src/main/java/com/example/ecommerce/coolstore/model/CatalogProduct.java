package com.example.ecommerce.coolstore.model;

/** Lightweight CoolStore catalog product (itemId keyed). */
public class CatalogProduct {
    private String itemId;
    private String name;
    private String desc;
    private double price;

    public CatalogProduct() {}

    public CatalogProduct(String itemId, String name, String desc, double price) {
        this.itemId = itemId;
        this.name = name;
        this.desc = desc;
        this.price = price;
    }

    public String getItemId() { return itemId; }
    public void setItemId(String itemId) { this.itemId = itemId; }
    public String getName() { return name; }
    public void setName(String name) { this.name = name; }
    public String getDesc() { return desc; }
    public void setDesc(String desc) { this.desc = desc; }
    public double getPrice() { return price; }
    public void setPrice(double price) { this.price = price; }
}
