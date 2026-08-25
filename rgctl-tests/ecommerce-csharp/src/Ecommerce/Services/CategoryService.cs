using Ecommerce.Dto;
using Ecommerce.Models;
using Ecommerce.Exceptions;
using Ecommerce.Repositories;

namespace Ecommerce.Services;

public class CategoryService
{
    private readonly CategoryRepository _categoryRepository;

    public CategoryService(CategoryRepository categoryRepository)
    {
        _categoryRepository = categoryRepository;
    }

    public async Task<List<CategoryDto>> FindAllAsync()
    {
        var categories = await _categoryRepository.FindAllAsync();
        return categories.Select(ToDto).ToList();
    }

    public async Task<CategoryDto> FindByIdAsync(long id)
    {
        return ToDto(await GetCategoryAsync(id));
    }

    public async Task<CategoryDto> CreateAsync(CategoryDto dto)
    {
        var category = new Category
        {
            Name = dto.Name,
            Description = dto.Description
        };
        category = await _categoryRepository.SaveAsync(category);
        return ToDto(category);
    }

    public async Task<Category> GetCategoryAsync(long id)
    {
        return await _categoryRepository.FindByIdAsync(id)
            ?? throw new ResourceNotFoundException($"Category not found: {id}");
    }

    private static CategoryDto ToDto(Category category) =>
        new(category.Id, category.Name, category.Description);
}
