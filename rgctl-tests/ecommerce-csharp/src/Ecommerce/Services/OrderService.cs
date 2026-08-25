using Ecommerce.Dto;
using Ecommerce.Models;
using Ecommerce.Exceptions;
using Ecommerce.Repositories;

namespace Ecommerce.Services;

public class OrderService
{
    private readonly OrderRepository _orderRepository;
    private readonly CartService _cartService;
    private readonly AuthService _authService;
    private readonly InventoryRepository _inventoryRepository;

    public OrderService(
        OrderRepository orderRepository,
        CartService cartService,
        AuthService authService,
        InventoryRepository inventoryRepository)
    {
        _orderRepository = orderRepository;
        _cartService = cartService;
        _authService = authService;
        _inventoryRepository = inventoryRepository;
    }

    public async Task<List<OrderDto>> FindMyOrdersAsync()
    {
        var user = await _authService.CurrentUserAsync();
        var orders = await _orderRepository.FindByUserIdOrderByCreatedAtDescAsync(user.Id);
        return orders.Select(ToDto).ToList();
    }

    public async Task<OrderDto> FindByIdAsync(long id)
    {
        var order = await GetOrderAsync(id);
        var user = await _authService.CurrentUserAsync();
        if (order.UserId != user.Id && user.Role != "ADMIN")
        {
            throw new ArgumentException("Access denied");
        }

        return ToDto(order);
    }

    public async Task<OrderDto> CheckoutAsync()
    {
        var user = await _authService.CurrentUserAsync();
        var cart = await _cartService.GetUserCartEntityAsync();
        if (cart.Items.Count == 0)
        {
            throw new ArgumentException("Cart is empty");
        }

        var order = new Order
        {
            UserId = user.Id,
            Status = "CONFIRMED"
        };
        var total = 0m;

        foreach (var cartItem in cart.Items)
        {
            var product = cartItem.Product;
            var inventory = await _inventoryRepository.FindByProductIdAsync(product.Id)
                ?? throw new ArgumentException($"Inventory not found for product: {product.Name}");

            if (inventory.Quantity < cartItem.Quantity)
            {
                throw new ArgumentException($"Insufficient stock for product: {product.Name}");
            }

            inventory.Quantity -= cartItem.Quantity;
            await _inventoryRepository.SaveAsync(inventory);

            order.Items.Add(new OrderItem
            {
                ProductId = product.Id,
                Quantity = cartItem.Quantity,
                UnitPrice = product.Price
            });
            total += product.Price * cartItem.Quantity;
        }

        order.TotalAmount = total;
        var saved = await _orderRepository.SaveAsync(order);
        await _cartService.ClearCartAsync();
        return ToDto(saved);
    }

    private async Task<Order> GetOrderAsync(long id)
    {
        return await _orderRepository.FindByIdAsync(id)
            ?? throw new ResourceNotFoundException($"Order not found: {id}");
    }

    private static OrderDto ToDto(Order order)
    {
        var items = order.Items.Select(item => new OrderItemDto(
            item.Id,
            item.ProductId,
            item.Product.Name,
            item.Quantity,
            item.UnitPrice)).ToList();

        return new OrderDto(order.Id, order.Status, order.TotalAmount, order.CreatedAt, items);
    }
}
