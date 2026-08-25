package service

import (
	"errors"

	"github.com/google/uuid"
	"github.com/sshaaf/ecommerce-go/internal/models"
	"github.com/sshaaf/ecommerce-go/internal/pkg/jwt"
	"github.com/sshaaf/ecommerce-go/internal/pkg/password"
	"github.com/sshaaf/ecommerce-go/internal/pkg/timeutil"
	"github.com/sshaaf/ecommerce-go/internal/repository"
)

type AuthService struct {
	users  *repository.UserRepository
	secret string
}

func NewAuthService(users *repository.UserRepository, secret string) *AuthService {
	return &AuthService{users: users, secret: secret}
}

type RegisterRequest struct {
	Email    string `json:"email" binding:"required,email"`
	Password string `json:"password" binding:"required,min=6"`
	Name     string `json:"name" binding:"required"`
}

type LoginRequest struct {
	Email    string `json:"email" binding:"required,email"`
	Password string `json:"password" binding:"required"`
}

type AuthResponse struct {
	Token  string `json:"token"`
	UserID string `json:"user_id"`
	Email  string `json:"email"`
	Name   string `json:"name"`
}

func (s *AuthService) Register(req RegisterRequest) (*AuthResponse, error) {
	existing, err := s.users.FindByEmail(req.Email)
	if err != nil {
		return nil, err
	}
	if existing != nil {
		return nil, NewConflict("email already registered")
	}

	hash, err := password.Hash(req.Password)
	if err != nil {
		return nil, err
	}

	user := &models.User{
		ID:           uuid.NewString(),
		Email:        req.Email,
		PasswordHash: hash,
		Name:         req.Name,
		Role:         "customer",
		CreatedAt:    timeutil.NowISO(),
	}
	if err := s.users.Create(user); err != nil {
		return nil, err
	}
	return s.authResponse(user)
}

func (s *AuthService) Login(req LoginRequest) (*AuthResponse, error) {
	user, err := s.users.FindByEmail(req.Email)
	if err != nil {
		return nil, err
	}
	if user == nil || !password.Verify(req.Password, user.PasswordHash) {
		return nil, NewUnauthorized()
	}
	return s.authResponse(user)
}

func (s *AuthService) authResponse(user *models.User) (*AuthResponse, error) {
	token, err := jwt.Sign(user.ID, user.Email, user.Role, s.secret)
	if err != nil {
		return nil, err
	}
	return &AuthResponse{
		Token:  token,
		UserID: user.ID,
		Email:  user.Email,
		Name:   user.Name,
	}, nil
}

func MapRepoError(err error) error {
	if errors.Is(err, repository.ErrNotFound) {
		return NewNotFound()
	}
	return err
}
