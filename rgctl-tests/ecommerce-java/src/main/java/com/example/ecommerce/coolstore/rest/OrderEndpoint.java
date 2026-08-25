package com.example.ecommerce.coolstore.rest;

import com.example.ecommerce.coolstore.model.CoolstoreOrder;
import com.example.ecommerce.coolstore.service.CoolstoreOrderService;
import org.springframework.http.HttpStatus;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;
import org.springframework.web.server.ResponseStatusException;

import java.util.List;

@RestController
@RequestMapping("/services/orders")
public class OrderEndpoint {

    private final CoolstoreOrderService orderService;

    public OrderEndpoint(CoolstoreOrderService orderService) {
        this.orderService = orderService;
    }

    @GetMapping
    public List<CoolstoreOrder> listAll() {
        return orderService.getOrders();
    }

    @GetMapping("/{orderId}")
    public CoolstoreOrder getOrder(@PathVariable long orderId) {
        return orderService
                .getOrderById(orderId)
                .orElseThrow(() -> new ResponseStatusException(HttpStatus.NOT_FOUND));
    }
}
