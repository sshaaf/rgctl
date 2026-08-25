namespace Ecommerce.Coolstore.Models;

/** Lightweight CoolStore catalog product (itemId keyed). */
public class CatalogProduct
{
    public string ItemId { get; set; } = string.Empty;
    public string Name { get; set; } = string.Empty;
    public string Desc { get; set; } = string.Empty;
    public double Price { get; set; }

    public CatalogProduct()
    {
    }

    public CatalogProduct(string itemId, string name, string desc, double price)
    {
        ItemId = itemId;
        Name = name;
        Desc = desc;
        Price = price;
    }
}
