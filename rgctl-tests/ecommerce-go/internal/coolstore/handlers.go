package coolstore

import (
	"net/http"
	"strconv"

	"github.com/gin-gonic/gin"
)

// Handlers exposes CoolStore /services/* endpoints.
type Handlers struct {
	Products *ProductService
	Cart     *ShoppingCartService
	Orders   *OrderService
}

func NewHandlers() *Handlers {
	products := NewProductService()
	promo := NewPromoService()
	shipping := NewShippingService()
	orders := NewOrderService()
	cart := NewShoppingCartService(products, promo, shipping, orders)
	return &Handlers{
		Products: products,
		Cart:     cart,
		Orders:   orders,
	}
}

func (h *Handlers) RegisterRoutes(r *gin.Engine) {
	products := r.Group("/services/products")
	{
		products.GET("", h.ListProducts)
		products.GET("/:itemId", h.GetProduct)
	}

	cart := r.Group("/services/cart")
	{
		cart.GET("/:cartId", h.GetCart)
		cart.POST("/checkout/:cartId", h.Checkout)
		cart.POST("/:cartId/:itemId/:quantity", h.AddItem)
		cart.DELETE("/:cartId/:itemId/:quantity", h.DeleteItem)
	}

	orders := r.Group("/services/orders")
	{
		orders.GET("", h.ListOrders)
		orders.GET("/:orderId", h.GetOrder)
	}
}

func (h *Handlers) ListProducts(c *gin.Context) {
	c.JSON(http.StatusOK, h.Products.GetProducts())
}

func (h *Handlers) GetProduct(c *gin.Context) {
	c.JSON(http.StatusOK, h.Products.GetProductByItemId(c.Param("itemId")))
}

func (h *Handlers) GetCart(c *gin.Context) {
	c.JSON(http.StatusOK, h.Cart.GetShoppingCart(c.Param("cartId")))
}

func (h *Handlers) Checkout(c *gin.Context) {
	c.JSON(http.StatusOK, h.Cart.CheckOutShoppingCart(c.Param("cartId")))
}

func (h *Handlers) AddItem(c *gin.Context) {
	cartID := c.Param("cartId")
	itemID := c.Param("itemId")
	quantity, err := strconv.Atoi(c.Param("quantity"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid quantity"})
		return
	}

	cart := h.Cart.GetShoppingCart(cartID)
	product := h.Cart.GetProduct(itemID)
	if product == nil {
		c.JSON(http.StatusOK, cart)
		return
	}
	sci := &ShoppingCartItem{
		Product:  product,
		Quantity: quantity,
		Price:    product.Price,
	}
	cart.AddShoppingCartItem(sci)
	h.Cart.PriceShoppingCart(cart)
	cart.ShoppingCartItemList = h.Cart.DedupeCartItems(cart.ShoppingCartItemList)
	h.Cart.PriceShoppingCart(cart)
	c.JSON(http.StatusOK, cart)
}

func (h *Handlers) DeleteItem(c *gin.Context) {
	cartID := c.Param("cartId")
	itemID := c.Param("itemId")
	quantity, err := strconv.Atoi(c.Param("quantity"))
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid quantity"})
		return
	}

	cart := h.Cart.GetShoppingCart(cartID)
	toRemove := make([]*ShoppingCartItem, 0)
	for _, sci := range cart.ShoppingCartItemList {
		if sci.Product != nil && itemID == sci.Product.ItemId {
			if quantity >= sci.Quantity {
				toRemove = append(toRemove, sci)
			} else {
				sci.Quantity -= quantity
			}
		}
	}
	for _, sci := range toRemove {
		cart.RemoveShoppingCartItem(sci)
	}
	h.Cart.PriceShoppingCart(cart)
	c.JSON(http.StatusOK, cart)
}

func (h *Handlers) ListOrders(c *gin.Context) {
	c.JSON(http.StatusOK, h.Orders.GetOrders())
}

func (h *Handlers) GetOrder(c *gin.Context) {
	orderID, err := strconv.ParseInt(c.Param("orderId"), 10, 64)
	if err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": "invalid orderId"})
		return
	}
	order, ok := h.Orders.GetOrderById(orderID)
	if !ok {
		c.JSON(http.StatusNotFound, gin.H{"error": "not found"})
		return
	}
	c.JSON(http.StatusOK, order)
}
