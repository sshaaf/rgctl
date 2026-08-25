package models

type Product struct {
	ID          string `gorm:"primaryKey" json:"id"`
	CategoryID  string `gorm:"not null;index" json:"category_id"`
	Name        string `gorm:"not null" json:"name"`
	Slug        string `gorm:"uniqueIndex;not null" json:"slug"`
	Description string `gorm:"not null;default:''" json:"description"`
	PriceCents  int64  `gorm:"not null" json:"price_cents"`
	Stock       int64  `gorm:"not null;default:0" json:"stock"`
	CreatedAt   string `gorm:"not null" json:"created_at"`
}

func (p *Product) SetCreatedAt(v string) { p.CreatedAt = v }
