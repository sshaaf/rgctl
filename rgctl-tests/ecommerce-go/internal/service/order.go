package service

import (
	"fmt"

	"github.com/google/uuid"
	"github.com/sshaaf/ecommerce-go/internal/models"
	"github.com/sshaaf/ecommerce-go/internal/pkg/timeutil"
	"github.com/sshaaf/ecommerce-go/internal/repository"
)

type OrderService struct {
	orders    *repository.OrderRepository
	cart      *repository.CartRepository
	products  *repository.ProductRepository
	inventory *repository.InventoryRepository
}

func NewOrderService(
	orders *repository.OrderRepository,
	cart *repository.CartRepository,
	products *repository.ProductRepository,
	inventory *repository.InventoryRepository,
) *OrderService {
	return &OrderService{
		orders:    orders,
		cart:      cart,
		products:  products,
		inventory: inventory,
	}
}

type OrderResponse struct {
	ID         string             `json:"id"`
	Status     string             `json:"status"`
	TotalCents int64              `json:"total_cents"`
	Items      []models.OrderItem `json:"items"`
}

func (s *OrderService) Checkout(userID string) (*OrderResponse, error) {
	items, err := s.cart.ListForUser(userID)
	if err != nil {
		return nil, err
	}
	if len(items) == 0 {
		return nil, NewBadRequest("cart is empty")
	}

	orderID := uuid.NewString()
	var total int64
	orderItems := make([]models.OrderItem, 0, len(items))

	for _, item := range items {
		product, err := s.products.FindByID(item.ProductID)
		if err != nil {
			return nil, MapRepoError(err)
		}
		if product.Stock < item.Quantity {
			return nil, NewBadRequest(fmt.Sprintf("insufficient stock for %s", product.Name))
		}
		total += product.PriceCents * item.Quantity
		orderItems = append(orderItems, models.OrderItem{
			ID:             uuid.NewString(),
			OrderID:        orderID,
			ProductID:      product.ID,
			Quantity:       item.Quantity,
			UnitPriceCents: product.PriceCents,
		})
	}

	order := &models.Order{
		ID:         orderID,
		UserID:     userID,
		Status:     "confirmed",
		TotalCents: total,
		CreatedAt:  timeutil.NowISO(),
	}
	if err := s.orders.Create(order); err != nil {
		return nil, err
	}
	for _, oi := range orderItems {
		if err := s.inventory.DecrementStock(oi.ProductID, oi.Quantity); err != nil {
			return nil, NewBadRequest(err.Error())
		}
		if err := s.orders.AddItem(&oi); err != nil {
			return nil, err
		}
	}
	if err := s.cart.Clear(userID); err != nil {
		return nil, err
	}
	return &OrderResponse{
		ID:         order.ID,
		Status:     order.Status,
		TotalCents: order.TotalCents,
		Items:      orderItems,
	}, nil
}

func (s *OrderService) List(userID string) ([]OrderResponse, error) {
	orders, err := s.orders.ListForUser(userID)
	if err != nil {
		return nil, err
	}
	out := make([]OrderResponse, 0, len(orders))
	for _, o := range orders {
		items, err := s.orders.ItemsForOrder(o.ID)
		if err != nil {
			return nil, err
		}
		out = append(out, OrderResponse{
			ID:         o.ID,
			Status:     o.Status,
			TotalCents: o.TotalCents,
			Items:      items,
		})
	}
	return out, nil
}

func (s *OrderService) Get(userID, id string) (*OrderResponse, error) {
	order, err := s.orders.FindByID(id)
	if err != nil {
		return nil, MapRepoError(err)
	}
	if order.UserID != userID {
		return nil, NewUnauthorized()
	}
	items, err := s.orders.ItemsForOrder(order.ID)
	if err != nil {
		return nil, err
	}
	return &OrderResponse{
		ID:         order.ID,
		Status:     order.Status,
		TotalCents: order.TotalCents,
		Items:      items,
	}, nil
}
