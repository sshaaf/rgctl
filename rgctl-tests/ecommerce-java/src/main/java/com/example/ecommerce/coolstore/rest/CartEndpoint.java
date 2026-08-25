package com.example.ecommerce.coolstore.rest;

import com.example.ecommerce.coolstore.model.CatalogProduct;
import com.example.ecommerce.coolstore.model.ShoppingCart;
import com.example.ecommerce.coolstore.model.ShoppingCartItem;
import com.example.ecommerce.coolstore.service.ShoppingCartService;
import org.springframework.web.bind.annotation.DeleteMapping;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

import java.util.ArrayList;
import java.util.List;

@RestController
@RequestMapping("/services/cart")
public class CartEndpoint {

    private final ShoppingCartService shoppingCartService;

    public CartEndpoint(ShoppingCartService shoppingCartService) {
        this.shoppingCartService = shoppingCartService;
    }

    @GetMapping("/{cartId}")
    public ShoppingCart getCart(@PathVariable String cartId) {
        return shoppingCartService.getShoppingCart(cartId);
    }

    @PostMapping("/checkout/{cartId}")
    public ShoppingCart checkout(@PathVariable String cartId) {
        return shoppingCartService.checkOutShoppingCart(cartId);
    }

    @PostMapping("/{cartId}/{itemId}/{quantity}")
    public ShoppingCart add(
            @PathVariable String cartId,
            @PathVariable String itemId,
            @PathVariable int quantity) {
        ShoppingCart cart = shoppingCartService.getShoppingCart(cartId);
        CatalogProduct product = shoppingCartService.getProduct(itemId);
        if (product == null) {
            return cart;
        }
        ShoppingCartItem sci = new ShoppingCartItem();
        sci.setProduct(product);
        sci.setQuantity(quantity);
        sci.setPrice(product.getPrice());
        cart.addShoppingCartItem(sci);
        shoppingCartService.priceShoppingCart(cart);
        cart.setShoppingCartItemList(shoppingCartService.dedupeCartItems(cart.getShoppingCartItemList()));
        shoppingCartService.priceShoppingCart(cart);
        return cart;
    }

    @DeleteMapping("/{cartId}/{itemId}/{quantity}")
    public ShoppingCart delete(
            @PathVariable String cartId,
            @PathVariable String itemId,
            @PathVariable int quantity) {
        ShoppingCart cart = shoppingCartService.getShoppingCart(cartId);
        List<ShoppingCartItem> toRemove = new ArrayList<>();
        for (ShoppingCartItem sci : cart.getShoppingCartItemList()) {
            if (sci.getProduct() != null && itemId.equals(sci.getProduct().getItemId())) {
                if (quantity >= sci.getQuantity()) {
                    toRemove.add(sci);
                } else {
                    sci.setQuantity(sci.getQuantity() - quantity);
                }
            }
        }
        toRemove.forEach(cart::removeShoppingCartItem);
        shoppingCartService.priceShoppingCart(cart);
        return cart;
    }
}
