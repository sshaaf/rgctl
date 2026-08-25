using Microsoft.AspNetCore.Mvc;

namespace Ecommerce.Controllers;

[ApiController]
public class HealthController : ControllerBase
{
    [HttpGet("/health")]
    public IActionResult Health() =>
        Ok(new { status = "UP" });
}
