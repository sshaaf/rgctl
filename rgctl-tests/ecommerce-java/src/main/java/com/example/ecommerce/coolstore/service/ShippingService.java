package com.example.ecommerce.coolstore.service;

import com.example.ecommerce.coolstore.model.ShoppingCart;
import org.springframework.stereotype.Service;

@Service
public class ShippingService {

    public double calculateShipping(ShoppingCart sc) {
        if (sc == null) {
            return 0;
        }
        double total = sc.getCartItemTotal();
        if (total >= 0 && total < 25) return 2.99;
        if (total >= 25 && total < 50) return 4.99;
        if (total >= 50 && total < 75) return 6.99;
        if (total >= 75 && total < 100) return 8.99;
        if (total >= 100) return 10.99;
        return 0;
    }

    public double calculateShippingInsurance(ShoppingCart sc) {
        if (sc == null) {
            return 0;
        }
        double total = sc.getCartItemTotal();
        if (total >= 25 && total < 100) return round2(total * 0.02);
        if (total >= 100) return round2(total * 0.015);
        return 0;
    }

    private static double round2(double v) {
        return Math.round(v * 100.0) / 100.0;
    }
}
