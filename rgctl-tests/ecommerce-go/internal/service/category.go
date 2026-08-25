package service

import (
	"github.com/google/uuid"
	"github.com/sshaaf/ecommerce-go/internal/models"
	"github.com/sshaaf/ecommerce-go/internal/pkg/timeutil"
	"github.com/sshaaf/ecommerce-go/internal/repository"
)

type CategoryService struct {
	categories *repository.CategoryRepository
}

func NewCategoryService(categories *repository.CategoryRepository) *CategoryService {
	return &CategoryService{categories: categories}
}

type CreateCategoryRequest struct {
	Name string `json:"name" binding:"required"`
	Slug string `json:"slug" binding:"required"`
}

type CategoryResponse struct {
	ID   string `json:"id"`
	Name string `json:"name"`
	Slug string `json:"slug"`
}

func (s *CategoryService) List() ([]CategoryResponse, error) {
	categories, err := s.categories.List()
	if err != nil {
		return nil, err
	}
	out := make([]CategoryResponse, len(categories))
	for i, c := range categories {
		out[i] = toCategoryResponse(c)
	}
	return out, nil
}

func (s *CategoryService) Get(id string) (*CategoryResponse, error) {
	category, err := s.categories.FindByID(id)
	if err != nil {
		return nil, MapRepoError(err)
	}
	resp := toCategoryResponse(*category)
	return &resp, nil
}

func (s *CategoryService) Create(req CreateCategoryRequest) (*CategoryResponse, error) {
	category := &models.Category{
		ID:        uuid.NewString(),
		Name:      req.Name,
		Slug:      req.Slug,
		CreatedAt: timeutil.NowISO(),
	}
	if err := s.categories.Create(category); err != nil {
		return nil, err
	}
	resp := toCategoryResponse(*category)
	return &resp, nil
}

func toCategoryResponse(c models.Category) CategoryResponse {
	return CategoryResponse{ID: c.ID, Name: c.Name, Slug: c.Slug}
}
