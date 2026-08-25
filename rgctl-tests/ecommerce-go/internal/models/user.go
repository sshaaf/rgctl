package models

type User struct {
	ID           string `gorm:"primaryKey" json:"id"`
	Email        string `gorm:"uniqueIndex;not null" json:"email"`
	PasswordHash string `gorm:"not null" json:"-"`
	Name         string `gorm:"not null" json:"name"`
	Role         string `gorm:"not null;default:customer" json:"role"`
	CreatedAt    string `gorm:"not null" json:"created_at"`
}

func (u *User) SetCreatedAt(v string) { u.CreatedAt = v }
