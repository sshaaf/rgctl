using System.Collections.Concurrent;
using Ecommerce.Coolstore.Models;

namespace Ecommerce.Coolstore.Services;

public class CoolstoreProductService
{
    private readonly ConcurrentDictionary<string, CatalogProduct> _catalog = new();

    public CoolstoreProductService()
    {
        Seed("329299", "Red Fedora", "Official Red Hat Fedora", 34.99);
        Seed("329199", "Forge Laptop Sticker", "JBoss Community sticker", 8.50);
        Seed("165613", "Solid Performance Polo", "Moisture-wicking polo", 17.80);
        Seed("165614", "Ogios T-shirt", "CoolStore tee", 11.50);
        Seed("165954", "Quarkus Stickers", "Pack of stickers", 9.99);
    }

    private void Seed(string id, string name, string desc, double price)
    {
        _catalog[id] = new CatalogProduct(id, name, desc, price);
    }

    public List<CatalogProduct> GetProducts() => _catalog.Values.ToList();

    public CatalogProduct? GetProductByItemId(string itemId) =>
        _catalog.TryGetValue(itemId, out var product) ? product : null;

    public Dictionary<string, CatalogProduct> AsMap() => new(_catalog);
}
