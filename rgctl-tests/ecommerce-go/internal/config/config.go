package config

import "os"

type Config struct {
	DatabasePath string
	JWTSecret    string
	BindAddr     string
}

func Load() *Config {
	return &Config{
		DatabasePath: envOr("DATABASE_PATH", "ecommerce.db"),
		JWTSecret:    envOr("JWT_SECRET", "dev-secret-change-me"),
		BindAddr:     envOr("BIND_ADDR", "0.0.0.0:8080"),
	}
}

func envOr(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}
