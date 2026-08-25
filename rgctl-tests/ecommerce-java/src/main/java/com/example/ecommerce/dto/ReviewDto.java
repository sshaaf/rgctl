package com.example.ecommerce.dto;

import java.time.Instant;

public record ReviewDto(Long id, Long userId, String userName, Long productId, int rating, String comment, Instant createdAt) {}
