using Ecommerce.Models;
using Microsoft.EntityFrameworkCore;
using Ecommerce.Data;

namespace Ecommerce.Repositories;

public class InventoryRepository
{
    private readonly EcommerceDbContext _db;

    public InventoryRepository(EcommerceDbContext db)
    {
        _db = db;
    }

    public Task<List<Inventory>> FindAllAsync() =>
        _db.Inventory
            .Include(i => i.Product)
            .OrderBy(i => i.Product.Name)
            .ToListAsync();

    public Task<Inventory?> FindByProductIdAsync(long productId) =>
        _db.Inventory
            .Include(i => i.Product)
            .FirstOrDefaultAsync(i => i.ProductId == productId);

    public async Task<Inventory> SaveAsync(Inventory inventory)
    {
        if (inventory.Id == 0)
        {
            _db.Inventory.Add(inventory);
        }
        else
        {
            _db.Inventory.Update(inventory);
        }

        await _db.SaveChangesAsync();

        return await FindByProductIdAsync(inventory.ProductId) ?? inventory;
    }
}
