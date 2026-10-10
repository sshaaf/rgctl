package com.example.clone;

/** Fixture: exact Type-1 duplicate of CloneA.normalizePayload. */
public class CloneB {
    public static String normalizePayload(String input) {
        if (input == null) {
            return "";
        }
        String trimmed = input.trim();
        if (trimmed.isEmpty()) {
            return "";
        }
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < trimmed.length(); i++) {
            char c = trimmed.charAt(i);
            if (Character.isLetterOrDigit(c) || c == '-' || c == '_') {
                sb.append(Character.toLowerCase(c));
            }
        }
        return sb.toString();
    }

    public static int uniqueHelperB(int x) {
        return x + 2;
    }
}
