package repository

import (
	"errors"

	"github.com/sshaaf/ecommerce-go/internal/models"
	"gorm.io/gorm"
)

type OrderRepository struct {
	db *gorm.DB
}

func NewOrderRepository(db *gorm.DB) *OrderRepository {
	return &OrderRepository{db: db}
}

func (r *OrderRepository) Create(order *models.Order) error {
	return r.db.Create(order).Error
}

func (r *OrderRepository) AddItem(item *models.OrderItem) error {
	return r.db.Create(item).Error
}

func (r *OrderRepository) ListForUser(userID string) ([]models.Order, error) {
	var orders []models.Order
	err := r.db.Where("user_id = ?", userID).Order("created_at desc").Find(&orders).Error
	return orders, err
}

func (r *OrderRepository) FindByID(id string) (*models.Order, error) {
	var order models.Order
	err := r.db.First(&order, "id = ?", id).Error
	if errors.Is(err, gorm.ErrRecordNotFound) {
		return nil, ErrNotFound
	}
	if err != nil {
		return nil, err
	}
	return &order, nil
}

func (r *OrderRepository) ItemsForOrder(orderID string) ([]models.OrderItem, error) {
	var items []models.OrderItem
	err := r.db.Where("order_id = ?", orderID).Find(&items).Error
	return items, err
}
