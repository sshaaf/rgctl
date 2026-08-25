using Ecommerce.Coolstore.Models;
using Ecommerce.Coolstore.Services;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace Ecommerce.Coolstore;

[ApiController]
[AllowAnonymous]
[Route("services/orders")]
public class OrderEndpoint : ControllerBase
{
    private readonly CoolstoreOrderService _orderService;

    public OrderEndpoint(CoolstoreOrderService orderService)
    {
        _orderService = orderService;
    }

    [HttpGet]
    public ActionResult<List<CoolstoreOrder>> ListAll() =>
        Ok(_orderService.GetOrders());

    [HttpGet("{orderId:long}")]
    public ActionResult<CoolstoreOrder> GetOrder(long orderId)
    {
        var order = _orderService.GetOrderById(orderId);
        if (order == null)
        {
            return NotFound();
        }

        return Ok(order);
    }
}
