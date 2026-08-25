using Ecommerce.Dto;
using Ecommerce.Models;
using Ecommerce.Exceptions;
using Ecommerce.Repositories;

namespace Ecommerce.Services;

public class ProductService
{
    private readonly ProductRepository _productRepository;
    private readonly CategoryService _categoryService;

    public ProductService(ProductRepository productRepository, CategoryService categoryService)
    {
        _productRepository = productRepository;
        _categoryService = categoryService;
    }

    public async Task<List<ProductDto>> FindAllAsync()
    {
        var products = await _productRepository.FindAllAsync();
        return products.Select(ToDto).ToList();
    }

    public async Task<List<ProductDto>> FindByCategoryAsync(long categoryId)
    {
        var products = await _productRepository.FindByCategoryIdAsync(categoryId);
        return products.Select(ToDto).ToList();
    }

    public async Task<ProductDto> FindByIdAsync(long id)
    {
        return ToDto(await GetProductAsync(id));
    }

    public async Task<ProductDto> CreateAsync(ProductDto dto)
    {
        _ = await _categoryService.GetCategoryAsync(dto.CategoryId);

        var product = new Product
        {
            Name = dto.Name,
            Description = dto.Description,
            Price = dto.Price,
            CategoryId = dto.CategoryId,
            Inventory = new Inventory
            {
                Quantity = dto.StockQuantity ?? 0
            }
        };

        product = await _productRepository.SaveAsync(product);
        return ToDto(await GetProductAsync(product.Id));
    }

    public async Task<Product> GetProductAsync(long id)
    {
        return await _productRepository.FindByIdAsync(id)
            ?? throw new ResourceNotFoundException($"Product not found: {id}");
    }

    private static ProductDto ToDto(Product product)
    {
        var stock = product.Inventory?.Quantity;
        return new ProductDto(
            product.Id,
            product.Name,
            product.Description,
            product.Price,
            product.CategoryId,
            product.Category.Name,
            stock);
    }
}
