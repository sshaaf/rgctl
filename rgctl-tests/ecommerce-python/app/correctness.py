"""Intentional call graph for rgctl expected-facts checks."""


def correctness_leaf() -> int:
    """Leaf — no outbound application calls."""
    return 42


def correctness_mid() -> int:
    """Mid — calls correctness_leaf."""
    return correctness_leaf() + 1


def correctness_root(flag: bool) -> int:
    """Root — calls correctness_mid and branches for a non-trivial CFG."""
    value = correctness_mid()
    if flag:
        return value * 2
    return value


def correctness_shared() -> int:
    """Shared sink for diamond topology QE."""
    return 1


def correctness_left() -> int:
    return correctness_shared()


def correctness_right() -> int:
    return correctness_shared()


def correctness_diamond() -> int:
    """Diamond root — dual callers into correctness_shared."""
    return correctness_left() + correctness_right()
