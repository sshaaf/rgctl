package models

type Category struct {
	ID        string `gorm:"primaryKey" json:"id"`
	Name      string `gorm:"not null" json:"name"`
	Slug      string `gorm:"uniqueIndex;not null" json:"slug"`
	CreatedAt string `gorm:"not null" json:"created_at"`
}

func (c *Category) SetCreatedAt(v string) { c.CreatedAt = v }
