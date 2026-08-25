package com.example.ecommerce.controller;

import com.example.ecommerce.dto.CartDto;
import com.example.ecommerce.service.CartService;
import org.springframework.web.bind.annotation.*;

@RestController
@RequestMapping("/api/cart")
public class CartController {

    private final CartService cartService;

    public CartController(CartService cartService) {
        this.cartService = cartService;
    }

    @GetMapping
    public CartDto getCart() {
        return cartService.getCart();
    }

    @PostMapping("/items")
    public CartDto addItem(@RequestParam Long productId, @RequestParam(defaultValue = "1") int quantity) {
        return cartService.addItem(productId, quantity);
    }

    @PutMapping("/items/{productId}")
    public CartDto updateItem(@PathVariable Long productId, @RequestParam int quantity) {
        return cartService.updateItem(productId, quantity);
    }

    @DeleteMapping
    public CartDto clearCart() {
        return cartService.clearCart();
    }
}
