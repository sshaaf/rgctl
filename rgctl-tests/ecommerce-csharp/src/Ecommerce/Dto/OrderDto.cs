namespace Ecommerce.Dto;

public record OrderDto(
    long Id,
    string Status,
    decimal TotalAmount,
    DateTime CreatedAt,
    List<OrderItemDto> Items);
