using Ecommerce.Coolstore.Models;
using Ecommerce.Coolstore.Services;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;

namespace Ecommerce.Coolstore;

[ApiController]
[AllowAnonymous]
[Route("services/cart")]
public class CartEndpoint : ControllerBase
{
    private readonly ShoppingCartService _shoppingCartService;

    public CartEndpoint(ShoppingCartService shoppingCartService)
    {
        _shoppingCartService = shoppingCartService;
    }

    [HttpGet("{cartId}")]
    public ActionResult<ShoppingCart> GetCart(string cartId) =>
        Ok(_shoppingCartService.GetShoppingCart(cartId));

    [HttpPost("checkout/{cartId}")]
    public ActionResult<ShoppingCart> Checkout(string cartId) =>
        Ok(_shoppingCartService.CheckOutShoppingCart(cartId));

    [HttpPost("{cartId}/{itemId}/{quantity:int}")]
    public ActionResult<ShoppingCart> Add(string cartId, string itemId, int quantity)
    {
        var cart = _shoppingCartService.GetShoppingCart(cartId);
        var product = _shoppingCartService.GetProduct(itemId);
        if (product == null)
        {
            return Ok(cart);
        }

        var sci = new ShoppingCartItem
        {
            Product = product,
            Quantity = quantity,
            Price = product.Price
        };
        cart.AddShoppingCartItem(sci);
        _shoppingCartService.PriceShoppingCart(cart);
        cart.ShoppingCartItemList = _shoppingCartService.DedupeCartItems(cart.ShoppingCartItemList);
        _shoppingCartService.PriceShoppingCart(cart);
        return Ok(cart);
    }

    [HttpDelete("{cartId}/{itemId}/{quantity:int}")]
    public ActionResult<ShoppingCart> Delete(string cartId, string itemId, int quantity)
    {
        var cart = _shoppingCartService.GetShoppingCart(cartId);
        var toRemove = new List<ShoppingCartItem>();
        foreach (var sci in cart.ShoppingCartItemList)
        {
            if (sci.Product != null && itemId == sci.Product.ItemId)
            {
                if (quantity >= sci.Quantity)
                {
                    toRemove.Add(sci);
                }
                else
                {
                    sci.Quantity -= quantity;
                }
            }
        }

        foreach (var sci in toRemove)
        {
            cart.RemoveShoppingCartItem(sci);
        }

        _shoppingCartService.PriceShoppingCart(cart);
        return Ok(cart);
    }
}
