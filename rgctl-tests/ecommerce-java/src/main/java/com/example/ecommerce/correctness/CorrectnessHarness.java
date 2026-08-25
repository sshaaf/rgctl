package com.example.ecommerce.correctness;

/**
 * Intentional static call graph for rgctl expected-facts checks.
 *
 * <p>Keep method names unique and stable. Prefer static calls so extraction does not
 * depend on Spring DI field resolution.
 */
public final class CorrectnessHarness {

    private CorrectnessHarness() {}

    /** Leaf — no outbound application calls. */
    public static int correctnessLeaf() {
        return 42;
    }

    /** Mid — calls {@link #correctnessLeaf()}. */
    public static int correctnessMid() {
        return correctnessLeaf() + 1;
    }

    /**
     * Root — calls {@link #correctnessMid()} and branches for a non-trivial CFG.
     *
     * @param flag branch selector
     * @return transformed mid value
     */
    public static int correctnessRoot(boolean flag) {
        int value = correctnessMid();
        if (flag) {
            return value * 2;
        }
        return value;
    }

    /** Shared sink for diamond topology QE. */
    public static int correctnessShared() {
        return 1;
    }

    public static int correctnessLeft() {
        return correctnessShared();
    }

    public static int correctnessRight() {
        return correctnessShared();
    }

    /** Diamond root — dual callers into {@link #correctnessShared()}. */
    public static int correctnessDiamond() {
        return correctnessLeft() + correctnessRight();
    }
}
