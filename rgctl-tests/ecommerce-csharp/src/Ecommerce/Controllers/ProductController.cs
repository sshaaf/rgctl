using Ecommerce.Dto;
using Ecommerce.Services;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace Ecommerce.Controllers;

[ApiController]
[Route("api/products")]
public class ProductController : ControllerBase
{
    private readonly ProductService _productService;
    private readonly ReviewService _reviewService;

    public ProductController(ProductService productService, ReviewService reviewService)
    {
        _productService = productService;
        _reviewService = reviewService;
    }

    [AllowAnonymous]
    [HttpGet]
    public async Task<ActionResult<List<ProductDto>>> List([FromQuery] long? categoryId)
    {
        if (categoryId.HasValue)
        {
            return Ok(await _productService.FindByCategoryAsync(categoryId.Value));
        }

        return Ok(await _productService.FindAllAsync());
    }

    [AllowAnonymous]
    [HttpGet("{id:long}")]
    public async Task<ActionResult<ProductDto>> Get(long id)
    {
        return Ok(await _productService.FindByIdAsync(id));
    }

    [Authorize(Roles = "ADMIN")]
    [HttpPost]
    public async Task<ActionResult<ProductDto>> Create([FromBody] ProductDto dto)
    {
        var created = await _productService.CreateAsync(dto);
        return CreatedAtAction(nameof(Get), new { id = created.Id }, created);
    }

    [AllowAnonymous]
    [HttpGet("{id:long}/reviews")]
    public async Task<ActionResult<List<ReviewDto>>> GetReviews(long id)
    {
        return Ok(await _reviewService.FindByProductAsync(id));
    }

    [Authorize]
    [HttpPost("{id:long}/reviews")]
    public async Task<ActionResult<ReviewDto>> CreateReview(
        long id,
        [FromQuery] int rating,
        [FromQuery] string? comment)
    {
        var review = await _reviewService.CreateAsync(id, rating, comment);
        return CreatedAtAction(nameof(GetReviews), new { id }, review);
    }
}
