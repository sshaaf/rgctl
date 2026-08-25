package models

type CartItem struct {
	UserID    string `gorm:"primaryKey" json:"user_id"`
	ProductID string `gorm:"primaryKey" json:"product_id"`
	Quantity  int64  `gorm:"not null" json:"quantity"`
}
