package repository

import (
	"errors"
	"fmt"

	"github.com/sshaaf/ecommerce-go/internal/models"
	"gorm.io/gorm"
)

type InventoryRepository struct {
	db *gorm.DB
}

func NewInventoryRepository(db *gorm.DB) *InventoryRepository {
	return &InventoryRepository{db: db}
}

func (r *InventoryRepository) DecrementStock(productID string, quantity int64) error {
	return r.db.Transaction(func(tx *gorm.DB) error {
		var product models.Product
		if err := tx.First(&product, "id = ?", productID).Error; err != nil {
			if errors.Is(err, gorm.ErrRecordNotFound) {
				return ErrNotFound
			}
			return err
		}
		if product.Stock < quantity {
			return fmt.Errorf("insufficient stock for %s", product.Name)
		}
		product.Stock -= quantity
		return tx.Save(&product).Error
	})
}

func (r *InventoryRepository) GetStock(productID string) (int64, error) {
	var product models.Product
	err := r.db.Select("stock").First(&product, "id = ?", productID).Error
	if errors.Is(err, gorm.ErrRecordNotFound) {
		return 0, ErrNotFound
	}
	if err != nil {
		return 0, err
	}
	return product.Stock, nil
}
