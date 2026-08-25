namespace Ecommerce.Dto;

public record ReviewDto(
    long Id,
    long UserId,
    string UserName,
    long ProductId,
    int Rating,
    string? Comment,
    DateTime CreatedAt);
