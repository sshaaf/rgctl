package models

type Review struct {
	ID        string `gorm:"primaryKey" json:"id"`
	ProductID string `gorm:"not null;index" json:"product_id"`
	UserID    string `gorm:"not null" json:"user_id"`
	Rating    int64  `gorm:"not null" json:"rating"`
	Comment   string `gorm:"not null;default:''" json:"comment"`
	CreatedAt string `gorm:"not null" json:"created_at"`
}

func (r *Review) SetCreatedAt(v string) { r.CreatedAt = v }
