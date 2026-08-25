/* Intentional call graph for rgctl expected-facts checks.
 * Avoid intermediate locals that some extractors mis-label as callers.
 */

int correctness_leaf(void) {
    return 42;
}

int correctness_mid(void) {
    return correctness_leaf() + 1;
}

int correctness_root(int flag) {
    if (flag) {
        return correctness_mid() * 2;
    }
    return correctness_mid();
}
