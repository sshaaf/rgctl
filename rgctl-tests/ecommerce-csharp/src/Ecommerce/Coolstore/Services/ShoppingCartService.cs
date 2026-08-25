using System.Collections.Concurrent;
using Ecommerce.Coolstore.Models;

namespace Ecommerce.Coolstore.Services;

public class ShoppingCartService
{
    private readonly CoolstoreProductService _productService;
    private readonly PromoService _promoService;
    private readonly ShippingService _shippingService;
    private readonly CoolstoreOrderService _orderService;
    private readonly ConcurrentDictionary<string, ShoppingCart> _carts = new();

    public ShoppingCartService(
        CoolstoreProductService productService,
        PromoService promoService,
        ShippingService shippingService,
        CoolstoreOrderService orderService)
    {
        _productService = productService;
        _promoService = promoService;
        _shippingService = shippingService;
        _orderService = orderService;
    }

    public ShoppingCart GetShoppingCart(string cartId) =>
        _carts.GetOrAdd(cartId, id => new ShoppingCart(id));

    public CatalogProduct? GetProduct(string itemId) =>
        _productService.GetProductByItemId(itemId);

    public ShoppingCart CheckOutShoppingCart(string cartId)
    {
        var cart = GetShoppingCart(cartId);
        PriceShoppingCart(cart);
        _orderService.Process(cart);
        cart.ResetShoppingCartItemList();
        PriceShoppingCart(cart);
        return cart;
    }

    /** Mutates ShoppingCart totals — primary CPG field-write site. */
    public void PriceShoppingCart(ShoppingCart? sc)
    {
        if (sc == null)
        {
            return;
        }

        InitShoppingCartForPricing(sc);

        if (sc.ShoppingCartItemList.Count > 0)
        {
            _promoService.ApplyCartItemPromotions(sc);

            foreach (var sci in sc.ShoppingCartItemList)
            {
                sc.CartItemPromoSavings += sci.PromoSavings * sci.Quantity;
                sc.CartItemTotal += sci.Price * sci.Quantity;
            }

            sc.ShippingTotal = _shippingService.CalculateShipping(sc);
            if (sc.CartItemTotal >= 25)
            {
                sc.ShippingTotal += _shippingService.CalculateShippingInsurance(sc);
            }
        }

        _promoService.ApplyShippingPromotions(sc);
        sc.CartTotal = sc.CartItemTotal + sc.ShippingTotal;
    }

    private void InitShoppingCartForPricing(ShoppingCart sc)
    {
        sc.CartItemTotal = 0;
        sc.CartItemPromoSavings = 0;
        sc.ShippingTotal = 0;
        sc.ShippingPromoSavings = 0;
        sc.CartTotal = 0;

        foreach (var sci in sc.ShoppingCartItemList)
        {
            if (sci.Product != null)
            {
                var p = GetProduct(sci.Product.ItemId);
                if (p != null)
                {
                    sci.Product = p;
                    sci.Price = p.Price;
                }
            }

            sci.PromoSavings = 0;
        }
    }

    public List<ShoppingCartItem> DedupeCartItems(List<ShoppingCartItem> cartItems)
    {
        var quantityMap = new Dictionary<string, int>();
        foreach (var sci in cartItems)
        {
            if (sci.Product == null)
            {
                continue;
            }

            var itemId = sci.Product.ItemId;
            quantityMap[itemId] = quantityMap.GetValueOrDefault(itemId) + sci.Quantity;
        }

        var result = new List<ShoppingCartItem>();
        foreach (var (itemId, quantity) in quantityMap)
        {
            var p = GetProduct(itemId);
            if (p == null)
            {
                continue;
            }

            result.Add(new ShoppingCartItem
            {
                Quantity = quantity,
                Price = p.Price,
                Product = p
            });
        }

        return result;
    }
}
