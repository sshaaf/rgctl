using System.Collections.Concurrent;
using Ecommerce.Coolstore.Models;

namespace Ecommerce.Coolstore.Services;

public class CoolstoreOrderService
{
    private long _seq = 1;
    private readonly ConcurrentDictionary<long, CoolstoreOrder> _orders = new();

    public CoolstoreOrder Process(ShoppingCart cart)
    {
        var order = new CoolstoreOrder
        {
            OrderId = Interlocked.Increment(ref _seq) - 1,
            CartId = cart.CartId,
            CartTotal = cart.CartTotal,
            Items = new List<CoolstoreOrderItem>()
        };

        foreach (var sci in cart.ShoppingCartItemList)
        {
            if (sci.Product != null)
            {
                order.Items.Add(new CoolstoreOrderItem(
                    sci.Product.ItemId,
                    sci.Quantity,
                    sci.Price));
            }
        }

        _orders[order.OrderId] = order;
        return order;
    }

    public List<CoolstoreOrder> GetOrders() => _orders.Values.ToList();

    public CoolstoreOrder? GetOrderById(long orderId) =>
        _orders.TryGetValue(orderId, out var order) ? order : null;
}
