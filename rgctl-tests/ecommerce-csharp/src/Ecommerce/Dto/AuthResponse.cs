namespace Ecommerce.Dto;

public record AuthResponse(string Token, long UserId, string Email, string Name, string Role);
