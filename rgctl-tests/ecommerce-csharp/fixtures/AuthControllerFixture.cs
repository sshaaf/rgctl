using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace Ecommerce.Fixtures.Controllers;

[Authorize]
[ApiController]
[Route("api/fixture")]
public class AuthControllerFixture : ControllerBase
{
    [HttpGet("ping")]
    public IActionResult Ping() => Ok();
}
