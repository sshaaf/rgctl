package com.example.clone;

/** Intentionally identical body under a test/ path for exclude filters. */
public class CloneTestDup {
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
}
