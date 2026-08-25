namespace Ecommerce.Coolstore.Models;

/** CoolStore-shaped cart with mutable pricing totals (CPG field-write target). */
public class ShoppingCart
{
    public string CartId { get; set; } = string.Empty;
    public double CartItemTotal { get; set; }
    public double CartItemPromoSavings { get; set; }
    public double ShippingTotal { get; set; }
    public double ShippingPromoSavings { get; set; }
    public double CartTotal { get; set; }
    public List<ShoppingCartItem> ShoppingCartItemList { get; set; } = new();

    public ShoppingCart()
    {
    }

    public ShoppingCart(string cartId)
    {
        CartId = cartId;
    }

    public void ResetShoppingCartItemList()
    {
        ShoppingCartItemList = new List<ShoppingCartItem>();
    }

    public void AddShoppingCartItem(ShoppingCartItem? sci)
    {
        if (sci != null)
        {
            ShoppingCartItemList.Add(sci);
        }
    }

    public bool RemoveShoppingCartItem(ShoppingCartItem? sci)
    {
        return sci != null && ShoppingCartItemList.Remove(sci);
    }
}
