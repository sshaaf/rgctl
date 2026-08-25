using Ecommerce.Dto;
using Ecommerce.Services;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace Ecommerce.Controllers;

[ApiController]
[Authorize]
[Route("api/cart")]
public class CartController : ControllerBase
{
    private readonly CartService _cartService;

    public CartController(CartService cartService)
    {
        _cartService = cartService;
    }

    [HttpGet]
    public async Task<ActionResult<CartDto>> GetCart()
    {
        return Ok(await _cartService.GetCartAsync());
    }

    [HttpPost("items")]
    public async Task<ActionResult<CartDto>> AddItem(
        [FromQuery] long productId,
        [FromQuery] int quantity = 1)
    {
        return Ok(await _cartService.AddItemAsync(productId, quantity));
    }

    [HttpDelete("items/{productId:long}")]
    public async Task<ActionResult<CartDto>> RemoveItem(long productId)
    {
        return Ok(await _cartService.RemoveItemAsync(productId));
    }
}
