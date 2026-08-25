package com.example.ecommerce.service;

import com.example.ecommerce.dto.ReviewDto;
import com.example.ecommerce.entity.Product;
import com.example.ecommerce.entity.Review;
import com.example.ecommerce.entity.User;
import com.example.ecommerce.repository.ReviewRepository;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.util.List;

@Service
public class ReviewService {

    private final ReviewRepository reviewRepository;
    private final ProductService productService;
    private final AuthService authService;

    public ReviewService(ReviewRepository reviewRepository, ProductService productService, AuthService authService) {
        this.reviewRepository = reviewRepository;
        this.productService = productService;
        this.authService = authService;
    }

    @Transactional(readOnly = true)
    public List<ReviewDto> findByProduct(Long productId) {
        productService.getProduct(productId);
        return reviewRepository.findByProductId(productId).stream().map(this::toDto).toList();
    }

    @Transactional
    public ReviewDto create(Long productId, int rating, String comment) {
        User user = authService.currentUser();
        Product product = productService.getProduct(productId);
        if (reviewRepository.findByUserIdAndProductId(user.getId(), productId).isPresent()) {
            throw new IllegalArgumentException("You have already reviewed this product");
        }
        Review review = new Review();
        review.setUser(user);
        review.setProduct(product);
        review.setRating(rating);
        review.setComment(comment);
        return toDto(reviewRepository.save(review));
    }

    private ReviewDto toDto(Review review) {
        return new ReviewDto(
                review.getId(),
                review.getUser().getId(),
                review.getUser().getName(),
                review.getProduct().getId(),
                review.getRating(),
                review.getComment(),
                review.getCreatedAt()
        );
    }
}
