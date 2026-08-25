package com.example.ecommerce.coolstore.model;

import java.util.ArrayList;
import java.util.List;

/** CoolStore-shaped cart with mutable pricing totals (CPG field-write target). */
public class ShoppingCart {
    private String cartId;
    private double cartItemTotal;
    private double cartItemPromoSavings;
    private double shippingTotal;
    private double shippingPromoSavings;
    private double cartTotal;
    private List<ShoppingCartItem> shoppingCartItemList = new ArrayList<>();

    public ShoppingCart() {}

    public ShoppingCart(String cartId) {
        this.cartId = cartId;
    }

    public String getCartId() { return cartId; }
    public void setCartId(String cartId) { this.cartId = cartId; }

    public List<ShoppingCartItem> getShoppingCartItemList() { return shoppingCartItemList; }
    public void setShoppingCartItemList(List<ShoppingCartItem> shoppingCartItemList) {
        this.shoppingCartItemList = shoppingCartItemList;
    }

    public void resetShoppingCartItemList() {
        shoppingCartItemList = new ArrayList<>();
    }

    public void addShoppingCartItem(ShoppingCartItem sci) {
        if (sci != null) {
            shoppingCartItemList.add(sci);
        }
    }

    public boolean removeShoppingCartItem(ShoppingCartItem sci) {
        return sci != null && shoppingCartItemList.remove(sci);
    }

    public double getCartItemTotal() { return cartItemTotal; }
    public void setCartItemTotal(double cartItemTotal) { this.cartItemTotal = cartItemTotal; }

    public double getCartItemPromoSavings() { return cartItemPromoSavings; }
    public void setCartItemPromoSavings(double cartItemPromoSavings) {
        this.cartItemPromoSavings = cartItemPromoSavings;
    }

    public double getShippingTotal() { return shippingTotal; }
    public void setShippingTotal(double shippingTotal) { this.shippingTotal = shippingTotal; }

    public double getShippingPromoSavings() { return shippingPromoSavings; }
    public void setShippingPromoSavings(double shippingPromoSavings) {
        this.shippingPromoSavings = shippingPromoSavings;
    }

    public double getCartTotal() { return cartTotal; }
    public void setCartTotal(double cartTotal) { this.cartTotal = cartTotal; }
}
