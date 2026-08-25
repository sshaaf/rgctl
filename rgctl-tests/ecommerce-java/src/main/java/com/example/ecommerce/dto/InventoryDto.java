package com.example.ecommerce.dto;

public record InventoryDto(Long id, Long productId, String productName, int quantity) {}
