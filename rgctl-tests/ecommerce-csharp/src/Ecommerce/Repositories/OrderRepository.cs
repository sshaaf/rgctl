using Ecommerce.Models;
using Microsoft.EntityFrameworkCore;
using Ecommerce.Data;

namespace Ecommerce.Repositories;

public class OrderRepository
{
    private readonly EcommerceDbContext _db;

    public OrderRepository(EcommerceDbContext db)
    {
        _db = db;
    }

    public Task<List<Order>> FindByUserIdOrderByCreatedAtDescAsync(long userId) =>
        _db.Orders
            .Include(o => o.Items)
            .ThenInclude(i => i.Product)
            .Where(o => o.UserId == userId)
            .OrderByDescending(o => o.CreatedAt)
            .ToListAsync();

    public Task<Order?> FindByIdAsync(long id) =>
        _db.Orders
            .Include(o => o.User)
            .Include(o => o.Items)
            .ThenInclude(i => i.Product)
            .FirstOrDefaultAsync(o => o.Id == id);

    public async Task<Order> SaveAsync(Order order)
    {
        if (order.Id == 0)
        {
            _db.Orders.Add(order);
        }
        else
        {
            _db.Orders.Update(order);
        }

        await _db.SaveChangesAsync();

        return await FindByIdAsync(order.Id) ?? order;
    }
}
