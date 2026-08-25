namespace Ecommerce.Correctness;

/// <summary>
/// CFG feature probes for rgctl expected-facts (C# lowering coverage).
/// </summary>
public static class CfgFeatures
{
    public static int CfgShortCircuit(bool a, bool b)
    {
        if (a && b)
        {
            return 1;
        }
        return 0;
    }

    public static int CfgForeach(int[] xs)
    {
        var total = 0;
        foreach (var v in xs)
        {
            total += v;
        }
        return total;
    }

    public static int CfgSwitchWhen(int x)
    {
        switch (x)
        {
            case 1:
                return 10;
            case int i when i > 0:
                return i;
            default:
                return 0;
        }
    }

    public static string CfgNullCoalesce(string? a, string b)
    {
        return a ?? b;
    }

    public static int CfgUsingDispose()
    {
        using var r = new System.IO.StringReader("");
        return 1;
    }

    public static async System.Threading.Tasks.Task<int> CfgAwait()
    {
        var t = await System.Threading.Tasks.Task.FromResult(1);
        return t;
    }
}
