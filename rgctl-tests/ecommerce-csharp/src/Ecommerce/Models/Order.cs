namespace Ecommerce.Models;

public class Order
{
    public long Id { get; set; }
    public long UserId { get; set; }
    public string Status { get; set; } = "PENDING";
    public decimal TotalAmount { get; set; }
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;

    public User User { get; set; } = null!;
    public ICollection<OrderItem> Items { get; set; } = new List<OrderItem>();
}
