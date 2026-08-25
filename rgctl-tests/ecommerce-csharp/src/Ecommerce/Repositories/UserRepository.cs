using Ecommerce.Models;
using Microsoft.EntityFrameworkCore;
using Ecommerce.Data;

namespace Ecommerce.Repositories;

public class UserRepository
{
    private readonly EcommerceDbContext _db;

    public UserRepository(EcommerceDbContext db)
    {
        _db = db;
    }

    public Task<User?> FindByEmailAsync(string email) =>
        _db.Users.FirstOrDefaultAsync(u => u.Email == email);

    public Task<bool> ExistsByEmailAsync(string email) =>
        _db.Users.AnyAsync(u => u.Email == email);

    public async Task<User?> FindByIdAsync(long id) =>
        await _db.Users.FindAsync(id);

    public async Task<User> SaveAsync(User user)
    {
        if (user.Id == 0)
        {
            _db.Users.Add(user);
        }
        else
        {
            _db.Users.Update(user);
        }

        await _db.SaveChangesAsync();
        return user;
    }
}
