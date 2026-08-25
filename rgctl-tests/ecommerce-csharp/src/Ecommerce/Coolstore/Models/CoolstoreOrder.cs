namespace Ecommerce.Coolstore.Models;

public class CoolstoreOrder
{
    public long OrderId { get; set; }
    public string CartId { get; set; } = string.Empty;
    public double CartTotal { get; set; }
    public List<CoolstoreOrderItem> Items { get; set; } = new();
}
