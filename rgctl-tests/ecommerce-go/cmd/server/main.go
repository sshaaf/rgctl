package main

import (
	"log"

	"github.com/gin-gonic/gin"
	"github.com/sshaaf/ecommerce-go/internal/config"
	"github.com/sshaaf/ecommerce-go/internal/database"
	"github.com/sshaaf/ecommerce-go/internal/handler"
)

func main() {
	cfg := config.Load()

	db, err := database.Connect(cfg.DatabasePath)
	if err != nil {
		log.Fatalf("database: %v", err)
	}
	if err := database.SeedDemo(db); err != nil {
		log.Fatalf("seed: %v", err)
	}

	gin.SetMode(gin.ReleaseMode)
	r := gin.New()
	r.Use(gin.Logger(), gin.Recovery())

	handlers := handler.NewHandlers(db, cfg)
	handlers.RegisterRoutes(r, cfg)

	log.Printf("ecommerce-go listening on %s", cfg.BindAddr)
	if err := r.Run(cfg.BindAddr); err != nil {
		log.Fatalf("server: %v", err)
	}
}
