package database

import (
	"github.com/google/uuid"
	"github.com/sshaaf/ecommerce-go/internal/models"
	"github.com/sshaaf/ecommerce-go/internal/pkg/timeutil"
	"gorm.io/gorm"
)

func SeedDemo(db *gorm.DB) error {
	var count int64
	if err := db.Model(&models.Category{}).Count(&count).Error; err != nil {
		return err
	}
	if count > 0 {
		return nil
	}

	catID := uuid.NewString()
	cat := models.Category{
		ID:        catID,
		Name:      "Electronics",
		Slug:      "electronics",
		CreatedAt: timeutil.NowISO(),
	}
	if err := db.Create(&cat).Error; err != nil {
		return err
	}

	products := []models.Product{
		{
			ID:          uuid.NewString(),
			CategoryID:  catID,
			Name:        "Wireless Headphones",
			Slug:        "wireless-headphones",
			Description: "Noise cancelling over-ear headphones",
			PriceCents:  12999,
			Stock:       50,
			CreatedAt:   timeutil.NowISO(),
		},
		{
			ID:          uuid.NewString(),
			CategoryID:  catID,
			Name:        "USB-C Hub",
			Slug:        "usb-c-hub",
			Description: "7-in-1 adapter",
			PriceCents:  4999,
			Stock:       120,
			CreatedAt:   timeutil.NowISO(),
		},
	}
	return db.Create(&products).Error
}
