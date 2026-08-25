using Ecommerce.Dto;
using Ecommerce.Models;
using Ecommerce.Repositories;

namespace Ecommerce.Services;

public class ReviewService
{
    private readonly ReviewRepository _reviewRepository;
    private readonly ProductService _productService;
    private readonly AuthService _authService;

    public ReviewService(
        ReviewRepository reviewRepository,
        ProductService productService,
        AuthService authService)
    {
        _reviewRepository = reviewRepository;
        _productService = productService;
        _authService = authService;
    }

    public async Task<List<ReviewDto>> FindByProductAsync(long productId)
    {
        _ = await _productService.GetProductAsync(productId);
        var reviews = await _reviewRepository.FindByProductIdAsync(productId);
        return reviews.Select(ToDto).ToList();
    }

    public async Task<ReviewDto> CreateAsync(long productId, int rating, string? comment)
    {
        var user = await _authService.CurrentUserAsync();
        var product = await _productService.GetProductAsync(productId);

        if (await _reviewRepository.FindByUserIdAndProductIdAsync(user.Id, productId) is not null)
        {
            throw new ArgumentException("You have already reviewed this product");
        }

        var review = new Review
        {
            UserId = user.Id,
            ProductId = product.Id,
            Rating = rating,
            Comment = comment
        };

        review = await _reviewRepository.SaveAsync(review);
        review.User = user;
        return ToDto(review);
    }

    private static ReviewDto ToDto(Review review) =>
        new(
            review.Id,
            review.UserId,
            review.User.Name,
            review.ProductId,
            review.Rating,
            review.Comment,
            review.CreatedAt);
}
