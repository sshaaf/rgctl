namespace Ecommerce.Dto;

public record CartItemDto(
    long Id,
    long ProductId,
    string ProductName,
    int Quantity,
    decimal UnitPrice,
    decimal LineTotal);
