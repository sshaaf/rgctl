package service

import (
	"github.com/sshaaf/ecommerce-go/internal/models"
	"github.com/sshaaf/ecommerce-go/internal/repository"
)

type CartService struct {
	cart     *repository.CartRepository
	products *repository.ProductRepository
}

func NewCartService(cart *repository.CartRepository, products *repository.ProductRepository) *CartService {
	return &CartService{cart: cart, products: products}
}

type AddCartItemRequest struct {
	ProductID string `json:"product_id" binding:"required"`
	Quantity  int64  `json:"quantity" binding:"required,min=1"`
}

type CartItemResponse struct {
	ProductID string `json:"product_id"`
	Quantity  int64  `json:"quantity"`
}

func (s *CartService) List(userID string) ([]CartItemResponse, error) {
	items, err := s.cart.ListForUser(userID)
	if err != nil {
		return nil, err
	}
	out := make([]CartItemResponse, len(items))
	for i, item := range items {
		out[i] = CartItemResponse{ProductID: item.ProductID, Quantity: item.Quantity}
	}
	return out, nil
}

func (s *CartService) Add(userID string, req AddCartItemRequest) (*CartItemResponse, error) {
	if _, err := s.products.FindByID(req.ProductID); err != nil {
		return nil, MapRepoError(err)
	}
	item := &models.CartItem{
		UserID:    userID,
		ProductID: req.ProductID,
		Quantity:  req.Quantity,
	}
	if err := s.cart.Upsert(item); err != nil {
		return nil, err
	}
	items, err := s.cart.ListForUser(userID)
	if err != nil {
		return nil, err
	}
	for _, i := range items {
		if i.ProductID == req.ProductID {
			return &CartItemResponse{ProductID: i.ProductID, Quantity: i.Quantity}, nil
		}
	}
	return &CartItemResponse{ProductID: req.ProductID, Quantity: req.Quantity}, nil
}

func (s *CartService) Remove(userID, productID string) error {
	return MapRepoError(s.cart.Remove(userID, productID))
}
