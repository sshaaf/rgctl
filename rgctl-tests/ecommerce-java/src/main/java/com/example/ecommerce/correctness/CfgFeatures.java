package com.example.ecommerce.correctness;

/**
 * CFG feature probes for rgctl expected-facts (Java lowering coverage).
 */
public final class CfgFeatures {

    private CfgFeatures() {}

    public static int cfgShortCircuit(boolean a, boolean b) {
        if (a && b) {
            return 1;
        }
        return 0;
    }

    public static int cfgEnhancedFor(int[] xs) {
        int total = 0;
        for (int v : xs) {
            total += v;
        }
        return total;
    }

    public static int cfgSwitchArrow(int x) {
        return switch (x) {
            case 1 -> 10;
            case 2 -> 20;
            default -> 0;
        };
    }

    public static int cfgTryWithResources() {
        try (Res r = new Res()) {
            return r.value();
        }
    }

    static final class Res implements AutoCloseable {
        int value() {
            return 1;
        }

        @Override
        public void close() {
            // synthetic close() for CFG
        }
    }
}
