package com.example.ecommerce.service;

import com.example.ecommerce.dto.OrderDto;
import com.example.ecommerce.dto.OrderItemDto;
import com.example.ecommerce.entity.*;
import com.example.ecommerce.exception.ResourceNotFoundException;
import com.example.ecommerce.repository.InventoryRepository;
import com.example.ecommerce.repository.OrderRepository;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.math.BigDecimal;
import java.util.ArrayList;
import java.util.List;

@Service
public class OrderService {

    private final OrderRepository orderRepository;
    private final CartService cartService;
    private final AuthService authService;
    private final InventoryRepository inventoryRepository;

    public OrderService(OrderRepository orderRepository, CartService cartService,
                        AuthService authService, InventoryRepository inventoryRepository) {
        this.orderRepository = orderRepository;
        this.cartService = cartService;
        this.authService = authService;
        this.inventoryRepository = inventoryRepository;
    }

    @Transactional(readOnly = true)
    public List<OrderDto> findMyOrders() {
        User user = authService.currentUser();
        return orderRepository.findByUserIdOrderByCreatedAtDesc(user.getId())
                .stream().map(this::toDto).toList();
    }

    @Transactional(readOnly = true)
    public OrderDto findById(Long id) {
        Order order = getOrder(id);
        User user = authService.currentUser();
        if (!order.getUser().getId().equals(user.getId()) && !"ADMIN".equals(user.getRole())) {
            throw new IllegalArgumentException("Access denied");
        }
        return toDto(order);
    }

    @Transactional
    public OrderDto checkout() {
        User user = authService.currentUser();
        Cart cart = cartService.getUserCartEntity();
        if (cart.getItems().isEmpty()) {
            throw new IllegalArgumentException("Cart is empty");
        }

        Order order = new Order();
        order.setUser(user);
        order.setStatus("CONFIRMED");
        BigDecimal total = BigDecimal.ZERO;

        for (CartItem cartItem : cart.getItems()) {
            Product product = cartItem.getProduct();
            Inventory inventory = inventoryRepository.findByProductId(product.getId())
                    .orElseThrow(() -> new IllegalArgumentException("Inventory not found for product: " + product.getName()));
            if (inventory.getQuantity() < cartItem.getQuantity()) {
                throw new IllegalArgumentException("Insufficient stock for product: " + product.getName());
            }
            inventory.setQuantity(inventory.getQuantity() - cartItem.getQuantity());
            inventoryRepository.save(inventory);

            OrderItem orderItem = new OrderItem();
            orderItem.setOrder(order);
            orderItem.setProduct(product);
            orderItem.setQuantity(cartItem.getQuantity());
            orderItem.setUnitPrice(product.getPrice());
            order.getItems().add(orderItem);
            total = total.add(product.getPrice().multiply(BigDecimal.valueOf(cartItem.getQuantity())));
        }

        order.setTotalAmount(total);
        Order saved = orderRepository.save(order);
        cartService.clearCart();
        return toDto(saved);
    }

    private Order getOrder(Long id) {
        return orderRepository.findById(id)
                .orElseThrow(() -> new ResourceNotFoundException("Order not found: " + id));
    }

    private OrderDto toDto(Order order) {
        List<OrderItemDto> items = new ArrayList<>();
        for (OrderItem item : order.getItems()) {
            items.add(new OrderItemDto(
                    item.getId(),
                    item.getProduct().getId(),
                    item.getProduct().getName(),
                    item.getQuantity(),
                    item.getUnitPrice()
            ));
        }
        return new OrderDto(order.getId(), order.getStatus(), order.getTotalAmount(), order.getCreatedAt(), items);
    }
}
