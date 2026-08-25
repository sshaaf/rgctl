namespace Ecommerce.Coolstore.Models;

public class CoolstoreOrderItem
{
    public string ProductId { get; set; } = string.Empty;
    public int Quantity { get; set; }
    public double Price { get; set; }

    public CoolstoreOrderItem()
    {
    }

    public CoolstoreOrderItem(string productId, int quantity, double price)
    {
        ProductId = productId;
        Quantity = quantity;
        Price = price;
    }
}
