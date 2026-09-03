namespace Ecommerce.Fixtures.Services;

public interface IOrderRepositoryFixture
{
    Task SaveAsync();
}

public class FieldInjectionFixture
{
    private readonly IOrderRepositoryFixture _repo;

    public FieldInjectionFixture(IOrderRepositoryFixture repo) => _repo = repo;

    public async Task RunAsync() => await _repo.SaveAsync();
}
