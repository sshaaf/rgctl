"""CFG feature probes for rgctl expected-facts (Python lowering coverage)."""


def cfg_short_circuit(a: bool, b: bool) -> int:
    if a and b:
        return 1
    return 0


def cfg_for_loop(xs: list[int]) -> int:
    total = 0
    for v in xs:
        total += v
    return total


def cfg_try_except() -> int:
    try:
        raise ValueError("boom")
    except ValueError:
        return 1
