namespace Ecommerce.Dto;

public record OrderItemDto(
    long Id,
    long ProductId,
    string ProductName,
    int Quantity,
    decimal UnitPrice);
