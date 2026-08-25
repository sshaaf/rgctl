package handler

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/sshaaf/ecommerce-go/internal/service"
)

type ProductHandler struct {
	products *service.ProductService
	reviews  *service.ReviewService
}

func NewProductHandler(products *service.ProductService, reviews *service.ReviewService) *ProductHandler {
	return &ProductHandler{products: products, reviews: reviews}
}

func (h *ProductHandler) List(c *gin.Context) {
	resp, err := h.products.List()
	if err != nil {
		handleError(c, err)
		return
	}
	c.JSON(http.StatusOK, resp)
}

func (h *ProductHandler) Get(c *gin.Context) {
	resp, err := h.products.Get(c.Param("id"))
	if err != nil {
		handleError(c, err)
		return
	}
	c.JSON(http.StatusOK, resp)
}

func (h *ProductHandler) Create(c *gin.Context) {
	var req service.CreateProductRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	resp, err := h.products.Create(req)
	if err != nil {
		handleError(c, err)
		return
	}
	c.JSON(http.StatusCreated, resp)
}

func (h *ProductHandler) ListReviews(c *gin.Context) {
	resp, err := h.reviews.List(c.Param("id"))
	if err != nil {
		handleError(c, err)
		return
	}
	c.JSON(http.StatusOK, resp)
}

func (h *ProductHandler) CreateReview(c *gin.Context) {
	var req service.CreateReviewRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	userID := c.GetString("userID")
	resp, err := h.reviews.Create(userID, c.Param("id"), req)
	if err != nil {
		handleError(c, err)
		return
	}
	c.JSON(http.StatusCreated, resp)
}
