using Ecommerce.Coolstore.Models;

namespace Ecommerce.Coolstore.Services;

public class PromoService
{
    private readonly Dictionary<string, double> _percentOffByItem = new()
    {
        ["329299"] = 0.25
    };

    public void ApplyCartItemPromotions(ShoppingCart shoppingCart)
    {
        if (shoppingCart.ShoppingCartItemList.Count == 0)
        {
            return;
        }

        foreach (var sci in shoppingCart.ShoppingCartItemList)
        {
            if (sci.Product == null)
            {
                continue;
            }

            if (_percentOffByItem.TryGetValue(sci.Product.ItemId, out var pct))
            {
                sci.PromoSavings = sci.Product.Price * pct * -1;
                sci.Price = sci.Product.Price * (1 - pct);
            }
        }
    }

    public void ApplyShippingPromotions(ShoppingCart shoppingCart)
    {
        if (shoppingCart.CartItemTotal >= 75)
        {
            shoppingCart.ShippingPromoSavings = shoppingCart.ShippingTotal * -1;
            shoppingCart.ShippingTotal = 0;
        }
    }
}
