namespace Ecommerce.Dto;

public record CartDto(long Id, List<CartItemDto> Items, decimal TotalAmount);
