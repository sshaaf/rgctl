package com.example.ecommerce.coolstore.service;

import com.example.ecommerce.coolstore.model.CatalogProduct;
import com.example.ecommerce.coolstore.model.ShoppingCart;
import com.example.ecommerce.coolstore.model.ShoppingCartItem;
import org.springframework.stereotype.Service;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

@Service
public class ShoppingCartService {

    private final CoolstoreProductService productService;
    private final PromoService promoService;
    private final ShippingService shippingService;
    private final CoolstoreOrderService orderService;
    private final Map<String, ShoppingCart> carts = new ConcurrentHashMap<>();

    public ShoppingCartService(
            CoolstoreProductService productService,
            PromoService promoService,
            ShippingService shippingService,
            CoolstoreOrderService orderService) {
        this.productService = productService;
        this.promoService = promoService;
        this.shippingService = shippingService;
        this.orderService = orderService;
    }

    public ShoppingCart getShoppingCart(String cartId) {
        return carts.computeIfAbsent(cartId, ShoppingCart::new);
    }

    public CatalogProduct getProduct(String itemId) {
        return productService.getProductByItemId(itemId);
    }

    public ShoppingCart checkOutShoppingCart(String cartId) {
        ShoppingCart cart = getShoppingCart(cartId);
        priceShoppingCart(cart);
        orderService.process(cart);
        cart.resetShoppingCartItemList();
        priceShoppingCart(cart);
        return cart;
    }

    /** Mutates ShoppingCart totals — primary CPG field-write site. */
    public void priceShoppingCart(ShoppingCart sc) {
        if (sc == null) {
            return;
        }
        initShoppingCartForPricing(sc);

        if (sc.getShoppingCartItemList() != null && !sc.getShoppingCartItemList().isEmpty()) {
            promoService.applyCartItemPromotions(sc);

            for (ShoppingCartItem sci : sc.getShoppingCartItemList()) {
                sc.setCartItemPromoSavings(
                        sc.getCartItemPromoSavings() + sci.getPromoSavings() * sci.getQuantity());
                sc.setCartItemTotal(sc.getCartItemTotal() + sci.getPrice() * sci.getQuantity());
            }

            sc.setShippingTotal(shippingService.calculateShipping(sc));
            if (sc.getCartItemTotal() >= 25) {
                sc.setShippingTotal(
                        sc.getShippingTotal() + shippingService.calculateShippingInsurance(sc));
            }
        }

        promoService.applyShippingPromotions(sc);
        sc.setCartTotal(sc.getCartItemTotal() + sc.getShippingTotal());
    }

    private void initShoppingCartForPricing(ShoppingCart sc) {
        sc.setCartItemTotal(0);
        sc.setCartItemPromoSavings(0);
        sc.setShippingTotal(0);
        sc.setShippingPromoSavings(0);
        sc.setCartTotal(0);

        for (ShoppingCartItem sci : sc.getShoppingCartItemList()) {
            if (sci.getProduct() != null) {
                CatalogProduct p = getProduct(sci.getProduct().getItemId());
                if (p != null) {
                    sci.setProduct(p);
                    sci.setPrice(p.getPrice());
                }
            }
            sci.setPromoSavings(0);
        }
    }

    public List<ShoppingCartItem> dedupeCartItems(List<ShoppingCartItem> cartItems) {
        Map<String, Integer> quantityMap = new HashMap<>();
        for (ShoppingCartItem sci : cartItems) {
            if (sci.getProduct() == null) {
                continue;
            }
            String itemId = sci.getProduct().getItemId();
            quantityMap.put(itemId, quantityMap.getOrDefault(itemId, 0) + sci.getQuantity());
        }
        List<ShoppingCartItem> result = new ArrayList<>();
        for (Map.Entry<String, Integer> e : quantityMap.entrySet()) {
            CatalogProduct p = getProduct(e.getKey());
            if (p == null) {
                continue;
            }
            ShoppingCartItem newItem = new ShoppingCartItem();
            newItem.setQuantity(e.getValue());
            newItem.setPrice(p.getPrice());
            newItem.setProduct(p);
            result.add(newItem);
        }
        return result;
    }
}
