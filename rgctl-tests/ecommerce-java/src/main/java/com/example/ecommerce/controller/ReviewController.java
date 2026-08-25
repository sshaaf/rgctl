package com.example.ecommerce.controller;

import com.example.ecommerce.dto.ReviewDto;
import com.example.ecommerce.service.ReviewService;
import org.springframework.http.HttpStatus;
import org.springframework.web.bind.annotation.*;

import java.util.List;

@RestController
@RequestMapping("/api/reviews")
public class ReviewController {

    private final ReviewService reviewService;

    public ReviewController(ReviewService reviewService) {
        this.reviewService = reviewService;
    }

    @GetMapping("/product/{productId}")
    public List<ReviewDto> byProduct(@PathVariable Long productId) {
        return reviewService.findByProduct(productId);
    }

    @PostMapping("/product/{productId}")
    @ResponseStatus(HttpStatus.CREATED)
    public ReviewDto create(@PathVariable Long productId,
                            @RequestParam int rating,
                            @RequestParam(required = false) String comment) {
        return reviewService.create(productId, rating, comment);
    }
}
