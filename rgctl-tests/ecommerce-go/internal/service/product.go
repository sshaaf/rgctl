package service

import (
	"github.com/google/uuid"
	"github.com/sshaaf/ecommerce-go/internal/models"
	"github.com/sshaaf/ecommerce-go/internal/pkg/timeutil"
	"github.com/sshaaf/ecommerce-go/internal/repository"
)

type ProductService struct {
	products *repository.ProductRepository
}

func NewProductService(products *repository.ProductRepository) *ProductService {
	return &ProductService{products: products}
}

type CreateProductRequest struct {
	CategoryID  string `json:"category_id" binding:"required"`
	Name        string `json:"name" binding:"required"`
	Slug        string `json:"slug" binding:"required"`
	Description string `json:"description"`
	PriceCents  int64  `json:"price_cents" binding:"required,min=0"`
	Stock       int64  `json:"stock" binding:"min=0"`
}

type ProductResponse struct {
	ID          string `json:"id"`
	CategoryID  string `json:"category_id"`
	Name        string `json:"name"`
	Slug        string `json:"slug"`
	Description string `json:"description"`
	PriceCents  int64  `json:"price_cents"`
	Stock       int64  `json:"stock"`
}

func (s *ProductService) List() ([]ProductResponse, error) {
	products, err := s.products.List()
	if err != nil {
		return nil, err
	}
	out := make([]ProductResponse, len(products))
	for i, p := range products {
		out[i] = toProductResponse(p)
	}
	return out, nil
}

func (s *ProductService) Get(id string) (*ProductResponse, error) {
	product, err := s.products.FindByID(id)
	if err != nil {
		return nil, MapRepoError(err)
	}
	resp := toProductResponse(*product)
	return &resp, nil
}

func (s *ProductService) Create(req CreateProductRequest) (*ProductResponse, error) {
	product := &models.Product{
		ID:          uuid.NewString(),
		CategoryID:  req.CategoryID,
		Name:        req.Name,
		Slug:        req.Slug,
		Description: req.Description,
		PriceCents:  req.PriceCents,
		Stock:       req.Stock,
		CreatedAt:   timeutil.NowISO(),
	}
	if err := s.products.Create(product); err != nil {
		return nil, err
	}
	resp := toProductResponse(*product)
	return &resp, nil
}

func toProductResponse(p models.Product) ProductResponse {
	return ProductResponse{
		ID:          p.ID,
		CategoryID:  p.CategoryID,
		Name:        p.Name,
		Slug:        p.Slug,
		Description: p.Description,
		PriceCents:  p.PriceCents,
		Stock:       p.Stock,
	}
}
