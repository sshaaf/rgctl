package com.example.ecommerce.dto;

import java.math.BigDecimal;

public record OrderItemDto(Long id, Long productId, String productName, int quantity, BigDecimal unitPrice) {}
