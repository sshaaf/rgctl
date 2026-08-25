using Ecommerce.Dto;
using Ecommerce.Services;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace Ecommerce.Controllers;

[ApiController]
[Authorize]
[Route("api/orders")]
public class OrderController : ControllerBase
{
    private readonly OrderService _orderService;

    public OrderController(OrderService orderService)
    {
        _orderService = orderService;
    }

    [HttpGet]
    public async Task<ActionResult<List<OrderDto>>> List()
    {
        return Ok(await _orderService.FindMyOrdersAsync());
    }

    [HttpGet("{id:long}")]
    public async Task<ActionResult<OrderDto>> Get(long id)
    {
        return Ok(await _orderService.FindByIdAsync(id));
    }

    [HttpPost]
    public async Task<ActionResult<OrderDto>> Checkout()
    {
        var order = await _orderService.CheckoutAsync();
        return CreatedAtAction(nameof(Get), new { id = order.Id }, order);
    }
}
