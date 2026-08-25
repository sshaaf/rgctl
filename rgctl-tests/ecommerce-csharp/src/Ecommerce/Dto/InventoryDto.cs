namespace Ecommerce.Dto;

public record InventoryDto(long Id, long ProductId, string ProductName, int Quantity);
