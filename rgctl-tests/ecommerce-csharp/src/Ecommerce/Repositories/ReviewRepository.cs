using Ecommerce.Models;
using Microsoft.EntityFrameworkCore;
using Ecommerce.Data;

namespace Ecommerce.Repositories;

public class ReviewRepository
{
    private readonly EcommerceDbContext _db;

    public ReviewRepository(EcommerceDbContext db)
    {
        _db = db;
    }

    public Task<List<Review>> FindByProductIdAsync(long productId) =>
        _db.Reviews
            .Include(r => r.User)
            .Where(r => r.ProductId == productId)
            .OrderByDescending(r => r.CreatedAt)
            .ToListAsync();

    public Task<Review?> FindByUserIdAndProductIdAsync(long userId, long productId) =>
        _db.Reviews.FirstOrDefaultAsync(r => r.UserId == userId && r.ProductId == productId);

    public async Task<Review> SaveAsync(Review review)
    {
        if (review.Id == 0)
        {
            _db.Reviews.Add(review);
        }
        else
        {
            _db.Reviews.Update(review);
        }

        await _db.SaveChangesAsync();

        return await _db.Reviews
            .Include(r => r.User)
            .FirstAsync(r => r.Id == review.Id);
    }
}
