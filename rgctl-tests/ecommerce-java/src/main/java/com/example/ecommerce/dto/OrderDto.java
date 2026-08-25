package com.example.ecommerce.dto;

import java.math.BigDecimal;
import java.time.Instant;
import java.util.List;

public record OrderDto(Long id, String status, BigDecimal totalAmount, Instant createdAt, List<OrderItemDto> items) {}
