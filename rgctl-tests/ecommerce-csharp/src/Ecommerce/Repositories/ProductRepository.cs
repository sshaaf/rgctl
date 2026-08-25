using Ecommerce.Models;
using Microsoft.EntityFrameworkCore;
using Ecommerce.Data;

namespace Ecommerce.Repositories;

public class ProductRepository
{
    private readonly EcommerceDbContext _db;

    public ProductRepository(EcommerceDbContext db)
    {
        _db = db;
    }

    public Task<List<Product>> FindAllAsync() =>
        _db.Products
            .Include(p => p.Category)
            .Include(p => p.Inventory)
            .OrderBy(p => p.Name)
            .ToListAsync();

    public Task<List<Product>> FindByCategoryIdAsync(long categoryId) =>
        _db.Products
            .Include(p => p.Category)
            .Include(p => p.Inventory)
            .Where(p => p.CategoryId == categoryId)
            .OrderBy(p => p.Name)
            .ToListAsync();

    public Task<Product?> FindByIdAsync(long id) =>
        _db.Products
            .Include(p => p.Category)
            .Include(p => p.Inventory)
            .FirstOrDefaultAsync(p => p.Id == id);

    public async Task<Product> SaveAsync(Product product)
    {
        if (product.Id == 0)
        {
            _db.Products.Add(product);
        }
        else
        {
            _db.Products.Update(product);
        }

        await _db.SaveChangesAsync();
        return product;
    }
}
