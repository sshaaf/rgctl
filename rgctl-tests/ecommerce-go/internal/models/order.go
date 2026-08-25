package models

type Order struct {
	ID         string `gorm:"primaryKey" json:"id"`
	UserID     string `gorm:"not null;index" json:"user_id"`
	Status     string `gorm:"not null" json:"status"`
	TotalCents int64  `gorm:"not null" json:"total_cents"`
	CreatedAt  string `gorm:"not null" json:"created_at"`
}

func (o *Order) SetCreatedAt(v string) { o.CreatedAt = v }

type OrderItem struct {
	ID              string `gorm:"primaryKey" json:"id"`
	OrderID         string `gorm:"not null;index" json:"order_id"`
	ProductID       string `gorm:"not null" json:"product_id"`
	Quantity        int64  `gorm:"not null" json:"quantity"`
	UnitPriceCents  int64  `gorm:"not null" json:"unit_price_cents"`
}
