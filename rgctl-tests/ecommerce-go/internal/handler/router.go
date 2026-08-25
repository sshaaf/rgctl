package handler

import (
	"github.com/gin-gonic/gin"
	"github.com/sshaaf/ecommerce-go/internal/config"
	"github.com/sshaaf/ecommerce-go/internal/coolstore"
	"github.com/sshaaf/ecommerce-go/internal/middleware"
	"github.com/sshaaf/ecommerce-go/internal/repository"
	"github.com/sshaaf/ecommerce-go/internal/service"
	"gorm.io/gorm"
)

type Handlers struct {
	Auth      *AuthHandler
	Category  *CategoryHandler
	Product   *ProductHandler
	Cart      *CartHandler
	Order     *OrderHandler
	Coolstore *coolstore.Handlers
}

func NewHandlers(db *gorm.DB, cfg *config.Config) *Handlers {
	userRepo := repository.NewUserRepository(db)
	categoryRepo := repository.NewCategoryRepository(db)
	productRepo := repository.NewProductRepository(db)
	cartRepo := repository.NewCartRepository(db)
	orderRepo := repository.NewOrderRepository(db)
	reviewRepo := repository.NewReviewRepository(db)
	inventoryRepo := repository.NewInventoryRepository(db)

	authSvc := service.NewAuthService(userRepo, cfg.JWTSecret)
	categorySvc := service.NewCategoryService(categoryRepo)
	productSvc := service.NewProductService(productRepo)
	cartSvc := service.NewCartService(cartRepo, productRepo)
	orderSvc := service.NewOrderService(orderRepo, cartRepo, productRepo, inventoryRepo)
	reviewSvc := service.NewReviewService(reviewRepo, productRepo)

	return &Handlers{
		Auth:      NewAuthHandler(authSvc),
		Category:  NewCategoryHandler(categorySvc),
		Product:   NewProductHandler(productSvc, reviewSvc),
		Cart:      NewCartHandler(cartSvc),
		Order:     NewOrderHandler(orderSvc),
		Coolstore: coolstore.NewHandlers(),
	}
}

func (h *Handlers) RegisterRoutes(r *gin.Engine, cfg *config.Config) {
	r.GET("/health", Health)

	api := r.Group("/api")
	{
		auth := api.Group("/auth")
		{
			auth.POST("/register", h.Auth.Register)
			auth.POST("/login", h.Auth.Login)
		}

		categories := api.Group("/categories")
		{
			categories.GET("", h.Category.List)
			categories.GET("/:id", h.Category.Get)
			categories.POST("", middleware.AuthRequired(cfg.JWTSecret), h.Category.Create)
		}

		products := api.Group("/products")
		{
			products.GET("", h.Product.List)
			products.POST("", middleware.AuthRequired(cfg.JWTSecret), h.Product.Create)
			products.GET("/:id", h.Product.Get)
			products.GET("/:id/reviews", h.Product.ListReviews)
			products.POST("/:id/reviews", middleware.AuthRequired(cfg.JWTSecret), h.Product.CreateReview)
		}

		cart := api.Group("/cart", middleware.AuthRequired(cfg.JWTSecret))
		{
			cart.GET("", h.Cart.List)
			cart.POST("/items", h.Cart.Add)
			cart.DELETE("/items/:product_id", h.Cart.Remove)
		}

		orders := api.Group("/orders", middleware.AuthRequired(cfg.JWTSecret))
		{
			orders.GET("", h.Order.List)
			orders.POST("", h.Order.Checkout)
			orders.GET("/:id", h.Order.Get)
		}
	}

	// CoolStore dual-API (/services/*) — unauthenticated in-memory store
	h.Coolstore.RegisterRoutes(r)
}
