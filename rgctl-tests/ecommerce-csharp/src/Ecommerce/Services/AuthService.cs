using System.Security.Claims;
using Ecommerce.Dto;
using Ecommerce.Models;
using Ecommerce.Repositories;
using Ecommerce.Security;

namespace Ecommerce.Services;

public class AuthService
{
    private readonly UserRepository _userRepository;
    private readonly CartRepository _cartRepository;
    private readonly JwtTokenProvider _jwtTokenProvider;
    private readonly IHttpContextAccessor _httpContextAccessor;

    public AuthService(
        UserRepository userRepository,
        CartRepository cartRepository,
        JwtTokenProvider jwtTokenProvider,
        IHttpContextAccessor httpContextAccessor)
    {
        _userRepository = userRepository;
        _cartRepository = cartRepository;
        _jwtTokenProvider = jwtTokenProvider;
        _httpContextAccessor = httpContextAccessor;
    }

    public async Task<AuthResponse> RegisterAsync(RegisterRequest request)
    {
        if (await _userRepository.ExistsByEmailAsync(request.Email))
        {
            throw new ArgumentException("Email already registered");
        }

        var user = new User
        {
            Email = request.Email,
            Password = BCrypt.Net.BCrypt.HashPassword(request.Password),
            Name = request.Name
        };
        user = await _userRepository.SaveAsync(user);

        var cart = new Cart { UserId = user.Id };
        await _cartRepository.SaveAsync(cart);

        var token = _jwtTokenProvider.GenerateToken(user.Email, user.Role);
        return new AuthResponse(token, user.Id, user.Email, user.Name, user.Role);
    }

    public async Task<AuthResponse> LoginAsync(LoginRequest request)
    {
        var user = await _userRepository.FindByEmailAsync(request.Email);
        if (user is null || !BCrypt.Net.BCrypt.Verify(request.Password, user.Password))
        {
            throw new UnauthorizedAccessException("Invalid email or password");
        }

        var token = _jwtTokenProvider.GenerateToken(user.Email, user.Role);
        return new AuthResponse(token, user.Id, user.Email, user.Name, user.Role);
    }

    public async Task<User> CurrentUserAsync()
    {
        var email = _httpContextAccessor.HttpContext?.User.FindFirstValue(ClaimTypes.Name)
            ?? _httpContextAccessor.HttpContext?.User.FindFirstValue(ClaimTypes.Email)
            ?? throw new UnauthorizedAccessException("Not authenticated");

        return await _userRepository.FindByEmailAsync(email)
            ?? throw new InvalidOperationException("Authenticated user not found");
    }
}
