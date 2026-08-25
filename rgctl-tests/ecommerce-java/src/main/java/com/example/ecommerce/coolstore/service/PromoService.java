package com.example.ecommerce.coolstore.service;

import com.example.ecommerce.coolstore.model.ShoppingCart;
import com.example.ecommerce.coolstore.model.ShoppingCartItem;
import org.springframework.stereotype.Service;

import java.util.HashMap;
import java.util.Map;

@Service
public class PromoService {

    private final Map<String, Double> percentOffByItem = new HashMap<>();

    public PromoService() {
        percentOffByItem.put("329299", 0.25);
    }

    public void applyCartItemPromotions(ShoppingCart shoppingCart) {
        if (shoppingCart == null || shoppingCart.getShoppingCartItemList().isEmpty()) {
            return;
        }
        for (ShoppingCartItem sci : shoppingCart.getShoppingCartItemList()) {
            if (sci.getProduct() == null) {
                continue;
            }
            Double pct = percentOffByItem.get(sci.getProduct().getItemId());
            if (pct != null) {
                sci.setPromoSavings(sci.getProduct().getPrice() * pct * -1);
                sci.setPrice(sci.getProduct().getPrice() * (1 - pct));
            }
        }
    }

    public void applyShippingPromotions(ShoppingCart shoppingCart) {
        if (shoppingCart == null) {
            return;
        }
        if (shoppingCart.getCartItemTotal() >= 75) {
            shoppingCart.setShippingPromoSavings(shoppingCart.getShippingTotal() * -1);
            shoppingCart.setShippingTotal(0);
        }
    }
}
