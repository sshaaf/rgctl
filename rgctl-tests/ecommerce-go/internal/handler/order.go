package handler

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/sshaaf/ecommerce-go/internal/service"
)

type OrderHandler struct {
	orders *service.OrderService
}

func NewOrderHandler(orders *service.OrderService) *OrderHandler {
	return &OrderHandler{orders: orders}
}

func (h *OrderHandler) Checkout(c *gin.Context) {
	userID := c.GetString("userID")
	resp, err := h.orders.Checkout(userID)
	if err != nil {
		handleError(c, err)
		return
	}
	c.JSON(http.StatusCreated, resp)
}

func (h *OrderHandler) List(c *gin.Context) {
	userID := c.GetString("userID")
	resp, err := h.orders.List(userID)
	if err != nil {
		handleError(c, err)
		return
	}
	c.JSON(http.StatusOK, resp)
}

func (h *OrderHandler) Get(c *gin.Context) {
	userID := c.GetString("userID")
	resp, err := h.orders.Get(userID, c.Param("id"))
	if err != nil {
		handleError(c, err)
		return
	}
	c.JSON(http.StatusOK, resp)
}
