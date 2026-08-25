using Ecommerce.Coolstore.Models;

namespace Ecommerce.Coolstore.Services;

public class ShippingService
{
    public double CalculateShipping(ShoppingCart sc)
    {
        var total = sc.CartItemTotal;
        if (total >= 0 && total < 25) return 2.99;
        if (total >= 25 && total < 50) return 4.99;
        if (total >= 50 && total < 75) return 6.99;
        if (total >= 75 && total < 100) return 8.99;
        if (total >= 100) return 10.99;
        return 0;
    }

    public double CalculateShippingInsurance(ShoppingCart sc)
    {
        var total = sc.CartItemTotal;
        if (total >= 25 && total < 100) return Round2(total * 0.02);
        if (total >= 100) return Round2(total * 0.015);
        return 0;
    }

    private static double Round2(double v) => Math.Round(v * 100.0) / 100.0;
}
