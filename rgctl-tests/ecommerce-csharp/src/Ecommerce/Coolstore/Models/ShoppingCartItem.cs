namespace Ecommerce.Coolstore.Models;

public class ShoppingCartItem
{
    public double Price { get; set; }
    public int Quantity { get; set; }
    public double PromoSavings { get; set; }
    public CatalogProduct? Product { get; set; }
}
