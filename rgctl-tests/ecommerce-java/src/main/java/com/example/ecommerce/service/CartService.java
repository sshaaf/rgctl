package com.example.ecommerce.service;

import com.example.ecommerce.dto.CartDto;
import com.example.ecommerce.dto.CartItemDto;
import com.example.ecommerce.entity.Cart;
import com.example.ecommerce.entity.CartItem;
import com.example.ecommerce.entity.Product;
import com.example.ecommerce.entity.User;
import com.example.ecommerce.exception.ResourceNotFoundException;
import com.example.ecommerce.repository.CartRepository;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.math.BigDecimal;
import java.util.ArrayList;
import java.util.List;

@Service
public class CartService {

    private final CartRepository cartRepository;
    private final ProductService productService;
    private final AuthService authService;

    public CartService(CartRepository cartRepository, ProductService productService, AuthService authService) {
        this.cartRepository = cartRepository;
        this.productService = productService;
        this.authService = authService;
    }

    @Transactional(readOnly = true)
    public CartDto getCart() {
        return toDto(getUserCart());
    }

    @Transactional
    public CartDto addItem(Long productId, int quantity) {
        Cart cart = getUserCart();
        Product product = productService.getProduct(productId);
        int available = product.getInventory() != null ? product.getInventory().getQuantity() : 0;
        if (available < quantity) {
            throw new IllegalArgumentException("Insufficient stock for product: " + product.getName());
        }

        CartItem existing = cart.getItems().stream()
                .filter(item -> item.getProduct().getId().equals(product.getId()))
                .findFirst()
                .orElse(null);

        if (existing != null) {
            existing.setQuantity(existing.getQuantity() + quantity);
        } else {
            CartItem item = new CartItem();
            item.setCart(cart);
            item.setProduct(product);
            item.setQuantity(quantity);
            cart.getItems().add(item);
        }
        return toDto(cartRepository.save(cart));
    }

    @Transactional
    public CartDto updateItem(Long productId, int quantity) {
        Cart cart = getUserCart();
        CartItem item = cart.getItems().stream()
                .filter(i -> i.getProduct().getId().equals(productId))
                .findFirst()
                .orElseThrow(() -> new ResourceNotFoundException("Cart item not found for product: " + productId));

        if (quantity <= 0) {
            cart.getItems().remove(item);
        } else {
            int available = item.getProduct().getInventory().getQuantity();
            if (available < quantity) {
                throw new IllegalArgumentException("Insufficient stock");
            }
            item.setQuantity(quantity);
        }
        return toDto(cartRepository.save(cart));
    }

    @Transactional
    public CartDto clearCart() {
        Cart cart = getUserCart();
        cart.getItems().clear();
        return toDto(cartRepository.save(cart));
    }

    @Transactional
    public Cart getUserCartEntity() {
        return getUserCart();
    }

    private Cart getUserCart() {
        User user = authService.currentUser();
        return cartRepository.findByUserId(user.getId())
                .orElseThrow(() -> new ResourceNotFoundException("Cart not found for user"));
    }

    private CartDto toDto(Cart cart) {
        List<CartItemDto> items = new ArrayList<>();
        BigDecimal total = BigDecimal.ZERO;
        for (CartItem item : cart.getItems()) {
            BigDecimal lineTotal = item.getProduct().getPrice().multiply(BigDecimal.valueOf(item.getQuantity()));
            total = total.add(lineTotal);
            items.add(new CartItemDto(
                    item.getId(),
                    item.getProduct().getId(),
                    item.getProduct().getName(),
                    item.getQuantity(),
                    item.getProduct().getPrice(),
                    lineTotal
            ));
        }
        return new CartDto(cart.getId(), items, total);
    }
}
