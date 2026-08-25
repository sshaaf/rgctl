package service

import (
	"github.com/google/uuid"
	"github.com/sshaaf/ecommerce-go/internal/models"
	"github.com/sshaaf/ecommerce-go/internal/pkg/timeutil"
	"github.com/sshaaf/ecommerce-go/internal/repository"
)

type ReviewService struct {
	reviews  *repository.ReviewRepository
	products *repository.ProductRepository
}

func NewReviewService(reviews *repository.ReviewRepository, products *repository.ProductRepository) *ReviewService {
	return &ReviewService{reviews: reviews, products: products}
}

type CreateReviewRequest struct {
	Rating  int64  `json:"rating" binding:"required,min=1,max=5"`
	Comment string `json:"comment"`
}

type ReviewResponse struct {
	ID        string `json:"id"`
	ProductID string `json:"product_id"`
	UserID    string `json:"user_id"`
	Rating    int64  `json:"rating"`
	Comment   string `json:"comment"`
}

func (s *ReviewService) List(productID string) ([]ReviewResponse, error) {
	if _, err := s.products.FindByID(productID); err != nil {
		return nil, MapRepoError(err)
	}
	reviews, err := s.reviews.ListForProduct(productID)
	if err != nil {
		return nil, err
	}
	out := make([]ReviewResponse, len(reviews))
	for i, r := range reviews {
		out[i] = toReviewResponse(r)
	}
	return out, nil
}

func (s *ReviewService) Create(userID, productID string, req CreateReviewRequest) (*ReviewResponse, error) {
	if _, err := s.products.FindByID(productID); err != nil {
		return nil, MapRepoError(err)
	}
	review := &models.Review{
		ID:        uuid.NewString(),
		ProductID: productID,
		UserID:    userID,
		Rating:    req.Rating,
		Comment:   req.Comment,
		CreatedAt: timeutil.NowISO(),
	}
	if err := s.reviews.Create(review); err != nil {
		return nil, err
	}
	resp := toReviewResponse(*review)
	return &resp, nil
}

func toReviewResponse(r models.Review) ReviewResponse {
	return ReviewResponse{
		ID:        r.ID,
		ProductID: r.ProductID,
		UserID:    r.UserID,
		Rating:    r.Rating,
		Comment:   r.Comment,
	}
}
