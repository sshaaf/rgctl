package com.example.ecommerce.controller;

import com.example.ecommerce.dto.InventoryDto;
import com.example.ecommerce.service.InventoryService;
import org.springframework.web.bind.annotation.*;

import java.util.List;

@RestController
@RequestMapping("/api/inventory")
public class InventoryController {

    private final InventoryService inventoryService;

    public InventoryController(InventoryService inventoryService) {
        this.inventoryService = inventoryService;
    }

    @GetMapping
    public List<InventoryDto> list() {
        return inventoryService.findAll();
    }

    @GetMapping("/product/{productId}")
    public InventoryDto byProduct(@PathVariable Long productId) {
        return inventoryService.findByProductId(productId);
    }

    @PutMapping("/product/{productId}")
    public InventoryDto update(@PathVariable Long productId, @RequestParam int quantity) {
        return inventoryService.updateQuantity(productId, quantity);
    }
}
