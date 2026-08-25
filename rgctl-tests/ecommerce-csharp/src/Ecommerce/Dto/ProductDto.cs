namespace Ecommerce.Dto;

public record ProductDto(
    long Id,
    string Name,
    string? Description,
    decimal Price,
    long CategoryId,
    string CategoryName,
    int? StockQuantity);
