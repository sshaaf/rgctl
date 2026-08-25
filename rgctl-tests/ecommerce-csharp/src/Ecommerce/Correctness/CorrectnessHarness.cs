namespace Ecommerce.Correctness;

/// <summary>
/// Intentional static call graph for rgctl expected-facts checks.
/// Prefer static calls so extraction does not depend on DI.
/// </summary>
public static class CorrectnessHarness
{
    /// <summary>Leaf — no outbound application calls.</summary>
    public static int CorrectnessLeaf() => 42;

    /// <summary>Mid — calls <see cref="CorrectnessLeaf"/>.</summary>
    public static int CorrectnessMid() => CorrectnessLeaf() + 1;

    /// <summary>Root — calls <see cref="CorrectnessMid"/> and branches for CFG.</summary>
    public static int CorrectnessRoot(bool flag)
    {
        var value = CorrectnessMid();
        if (flag)
        {
            return value * 2;
        }
        return value;
    }
}
