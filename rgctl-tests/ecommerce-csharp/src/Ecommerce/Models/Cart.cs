namespace Ecommerce.Models;

public class Cart
{
    public long Id { get; set; }
    public long UserId { get; set; }

    public User User { get; set; } = null!;
    public ICollection<CartItem> Items { get; set; } = new List<CartItem>();
}
