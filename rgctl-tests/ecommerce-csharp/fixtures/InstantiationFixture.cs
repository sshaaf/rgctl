namespace Ecommerce.Fixtures.Dto;

public class OrderDtoFixture
{
    public int Id { get; set; }
}

namespace Ecommerce.Fixtures.Services;

public class InstantiationFixture
{
    public OrderDtoFixture Create() => new OrderDtoFixture();
}
