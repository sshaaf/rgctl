package handler

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/sshaaf/ecommerce-go/internal/service"
)

type CartHandler struct {
	cart *service.CartService
}

func NewCartHandler(cart *service.CartService) *CartHandler {
	return &CartHandler{cart: cart}
}

func (h *CartHandler) List(c *gin.Context) {
	userID := c.GetString("userID")
	resp, err := h.cart.List(userID)
	if err != nil {
		handleError(c, err)
		return
	}
	c.JSON(http.StatusOK, resp)
}

func (h *CartHandler) Add(c *gin.Context) {
	var req service.AddCartItemRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}
	userID := c.GetString("userID")
	resp, err := h.cart.Add(userID, req)
	if err != nil {
		handleError(c, err)
		return
	}
	c.JSON(http.StatusOK, resp)
}

func (h *CartHandler) Remove(c *gin.Context) {
	userID := c.GetString("userID")
	if err := h.cart.Remove(userID, c.Param("product_id")); err != nil {
		handleError(c, err)
		return
	}
	c.Status(http.StatusNoContent)
}
