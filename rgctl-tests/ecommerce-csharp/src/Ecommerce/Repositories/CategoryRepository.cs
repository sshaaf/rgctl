using Ecommerce.Models;
using Microsoft.EntityFrameworkCore;
using Ecommerce.Data;

namespace Ecommerce.Repositories;

public class CategoryRepository
{
    private readonly EcommerceDbContext _db;

    public CategoryRepository(EcommerceDbContext db)
    {
        _db = db;
    }

    public Task<List<Category>> FindAllAsync() =>
        _db.Categories.OrderBy(c => c.Name).ToListAsync();

    public async Task<Category?> FindByIdAsync(long id) =>
        await _db.Categories.FindAsync(id);

    public Task<Category?> FindByNameAsync(string name) =>
        _db.Categories.FirstOrDefaultAsync(c => c.Name == name);

    public async Task<Category> SaveAsync(Category category)
    {
        if (category.Id == 0)
        {
            _db.Categories.Add(category);
        }
        else
        {
            _db.Categories.Update(category);
        }

        await _db.SaveChangesAsync();
        return category;
    }

    public Task<int> CountAsync() =>
        _db.Categories.CountAsync();
}
