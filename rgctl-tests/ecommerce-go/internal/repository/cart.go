package repository

import (
	"errors"

	"github.com/sshaaf/ecommerce-go/internal/models"
	"gorm.io/gorm"
)

type CartRepository struct {
	db *gorm.DB
}

func NewCartRepository(db *gorm.DB) *CartRepository {
	return &CartRepository{db: db}
}

func (r *CartRepository) ListForUser(userID string) ([]models.CartItem, error) {
	var items []models.CartItem
	err := r.db.Where("user_id = ?", userID).Find(&items).Error
	return items, err
}

func (r *CartRepository) Upsert(item *models.CartItem) error {
	var existing models.CartItem
	err := r.db.Where("user_id = ? AND product_id = ?", item.UserID, item.ProductID).First(&existing).Error
	if errors.Is(err, gorm.ErrRecordNotFound) {
		return r.db.Create(item).Error
	}
	if err != nil {
		return err
	}
	existing.Quantity += item.Quantity
	return r.db.Save(&existing).Error
}

func (r *CartRepository) Remove(userID, productID string) error {
	result := r.db.Where("user_id = ? AND product_id = ?", userID, productID).Delete(&models.CartItem{})
	if result.Error != nil {
		return result.Error
	}
	if result.RowsAffected == 0 {
		return ErrNotFound
	}
	return nil
}

func (r *CartRepository) Clear(userID string) error {
	return r.db.Where("user_id = ?", userID).Delete(&models.CartItem{}).Error
}
