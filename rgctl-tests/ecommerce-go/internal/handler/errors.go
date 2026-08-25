package handler

import (
	"errors"
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/sshaaf/ecommerce-go/internal/service"
)

func handleError(c *gin.Context, err error) {
	var appErr *service.AppError
	if errors.As(err, &appErr) {
		switch {
		case errors.Is(appErr.Code, service.ErrNotFound):
			c.JSON(http.StatusNotFound, gin.H{"error": appErr.Error()})
		case errors.Is(appErr.Code, service.ErrUnauthorized):
			c.JSON(http.StatusUnauthorized, gin.H{"error": "unauthorized"})
		case errors.Is(appErr.Code, service.ErrConflict):
			c.JSON(http.StatusConflict, gin.H{"error": appErr.Error()})
		case errors.Is(appErr.Code, service.ErrBadRequest):
			c.JSON(http.StatusBadRequest, gin.H{"error": appErr.Error()})
		default:
			c.JSON(http.StatusInternalServerError, gin.H{"error": "internal server error"})
		}
		return
	}
	c.JSON(http.StatusInternalServerError, gin.H{"error": "internal server error"})
}
