using Ecommerce.Models;
using Microsoft.EntityFrameworkCore;
using Ecommerce.Data;

namespace Ecommerce.Repositories;

public class CartRepository
{
    private readonly EcommerceDbContext _db;

    public CartRepository(EcommerceDbContext db)
    {
        _db = db;
    }

    public Task<Cart?> FindByUserIdAsync(long userId) =>
        _db.Carts
            .Include(c => c.Items)
            .ThenInclude(i => i.Product)
            .ThenInclude(p => p.Inventory)
            .FirstOrDefaultAsync(c => c.UserId == userId);

    public async Task<Cart> SaveAsync(Cart cart)
    {
        if (cart.Id == 0)
        {
            _db.Carts.Add(cart);
        }

        await _db.SaveChangesAsync();

        return await FindByUserIdAsync(cart.UserId)
            ?? cart;
    }
}
