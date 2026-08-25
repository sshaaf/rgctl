package repository

import (
	"errors"

	"github.com/sshaaf/ecommerce-go/internal/models"
	"gorm.io/gorm"
)

type ReviewRepository struct {
	db *gorm.DB
}

func NewReviewRepository(db *gorm.DB) *ReviewRepository {
	return &ReviewRepository{db: db}
}

func (r *ReviewRepository) ListForProduct(productID string) ([]models.Review, error) {
	var reviews []models.Review
	err := r.db.Where("product_id = ?", productID).Order("created_at desc").Find(&reviews).Error
	return reviews, err
}

func (r *ReviewRepository) Create(review *models.Review) error {
	return r.db.Create(review).Error
}

func (r *ReviewRepository) FindByUserAndProduct(userID, productID string) (*models.Review, error) {
	var review models.Review
	err := r.db.Where("user_id = ? AND product_id = ?", userID, productID).First(&review).Error
	if errors.Is(err, gorm.ErrRecordNotFound) {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	return &review, nil
}
