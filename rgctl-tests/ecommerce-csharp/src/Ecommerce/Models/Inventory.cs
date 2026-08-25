namespace Ecommerce.Models;

public class Inventory
{
    public long Id { get; set; }
    public long ProductId { get; set; }
    public int Quantity { get; set; }

    public Product Product { get; set; } = null!;
}
