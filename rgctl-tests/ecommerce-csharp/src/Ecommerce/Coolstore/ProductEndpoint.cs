using Ecommerce.Coolstore.Models;
using Ecommerce.Coolstore.Services;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace Ecommerce.Coolstore;

[ApiController]
[AllowAnonymous]
[Route("services/products")]
public class ProductEndpoint : ControllerBase
{
    private readonly CoolstoreProductService _productService;

    public ProductEndpoint(CoolstoreProductService productService)
    {
        _productService = productService;
    }

    [HttpGet]
    public ActionResult<List<CatalogProduct>> ListAll() =>
        Ok(_productService.GetProducts());

    [HttpGet("{itemId}")]
    public ActionResult<CatalogProduct?> GetProduct(string itemId) =>
        Ok(_productService.GetProductByItemId(itemId));
}
