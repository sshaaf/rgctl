using Ecommerce.Dto;
using Ecommerce.Models;
using Ecommerce.Exceptions;
using Ecommerce.Repositories;

namespace Ecommerce.Services;

public class CartService
{
    private readonly CartRepository _cartRepository;
    private readonly ProductService _productService;
    private readonly AuthService _authService;

    public CartService(
        CartRepository cartRepository,
        ProductService productService,
        AuthService authService)
    {
        _cartRepository = cartRepository;
        _productService = productService;
        _authService = authService;
    }

    public async Task<CartDto> GetCartAsync()
    {
        return ToDto(await GetUserCartAsync());
    }

    public async Task<CartDto> AddItemAsync(long productId, int quantity)
    {
        var cart = await GetUserCartAsync();
        var product = await _productService.GetProductAsync(productId);
        var available = product.Inventory?.Quantity ?? 0;
        if (available < quantity)
        {
            throw new ArgumentException($"Insufficient stock for product: {product.Name}");
        }

        var existing = cart.Items.FirstOrDefault(i => i.ProductId == product.Id);
        if (existing is not null)
        {
            existing.Quantity += quantity;
        }
        else
        {
            cart.Items.Add(new CartItem
            {
                CartId = cart.Id,
                ProductId = product.Id,
                Quantity = quantity
            });
        }

        cart = await _cartRepository.SaveAsync(cart);
        return ToDto(cart);
    }

    public async Task<CartDto> RemoveItemAsync(long productId)
    {
        var cart = await GetUserCartAsync();
        var item = cart.Items.FirstOrDefault(i => i.ProductId == productId)
            ?? throw new ResourceNotFoundException($"Cart item not found for product: {productId}");

        cart.Items.Remove(item);
        await _cartRepository.SaveAsync(cart);
        return ToDto(await GetUserCartAsync());
    }

    public async Task<CartDto> ClearCartAsync()
    {
        var cart = await GetUserCartAsync();
        cart.Items.Clear();
        cart = await _cartRepository.SaveAsync(cart);
        return ToDto(cart);
    }

    public async Task<Cart> GetUserCartEntityAsync()
    {
        return await GetUserCartAsync();
    }

    private async Task<Cart> GetUserCartAsync()
    {
        var user = await _authService.CurrentUserAsync();
        return await _cartRepository.FindByUserIdAsync(user.Id)
            ?? throw new ResourceNotFoundException("Cart not found for user");
    }

    private static CartDto ToDto(Cart cart)
    {
        var items = new List<CartItemDto>();
        var total = 0m;

        foreach (var item in cart.Items)
        {
            var lineTotal = item.Product.Price * item.Quantity;
            total += lineTotal;
            items.Add(new CartItemDto(
                item.Id,
                item.ProductId,
                item.Product.Name,
                item.Quantity,
                item.Product.Price,
                lineTotal));
        }

        return new CartDto(cart.Id, items, total);
    }
}
