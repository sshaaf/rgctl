using Ecommerce.Dto;
using Ecommerce.Models;
using Ecommerce.Exceptions;
using Ecommerce.Repositories;

namespace Ecommerce.Services;

public class InventoryService
{
    private readonly InventoryRepository _inventoryRepository;
    private readonly ProductService _productService;

    public InventoryService(InventoryRepository inventoryRepository, ProductService productService)
    {
        _inventoryRepository = inventoryRepository;
        _productService = productService;
    }

    public async Task<List<InventoryDto>> FindAllAsync()
    {
        var inventory = await _inventoryRepository.FindAllAsync();
        return inventory.Select(ToDto).ToList();
    }

    public async Task<InventoryDto> FindByProductIdAsync(long productId)
    {
        return ToDto(await GetByProductIdAsync(productId));
    }

    public async Task<InventoryDto> UpdateQuantityAsync(long productId, int quantity)
    {
        var inventory = await GetByProductIdAsync(productId);
        inventory.Quantity = quantity;
        inventory = await _inventoryRepository.SaveAsync(inventory);
        return ToDto(inventory);
    }

    private async Task<Inventory> GetByProductIdAsync(long productId)
    {
        _ = await _productService.GetProductAsync(productId);
        return await _inventoryRepository.FindByProductIdAsync(productId)
            ?? throw new ResourceNotFoundException($"Inventory not found for product: {productId}");
    }

    private static InventoryDto ToDto(Inventory inventory) =>
        new(inventory.Id, inventory.ProductId, inventory.Product.Name, inventory.Quantity);
}
