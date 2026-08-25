package com.example.ecommerce.dto;

public record AuthResponse(String token, Long userId, String email, String name, String role) {}
