package service

import "errors"

var (
	ErrNotFound      = errors.New("not found")
	ErrUnauthorized  = errors.New("unauthorized")
	ErrConflict      = errors.New("conflict")
	ErrBadRequest    = errors.New("bad request")
)

type AppError struct {
	Code    error
	Message string
}

func (e *AppError) Error() string {
	if e.Message != "" {
		return e.Message
	}
	return e.Code.Error()
}

func NewBadRequest(msg string) *AppError {
	return &AppError{Code: ErrBadRequest, Message: msg}
}

func NewConflict(msg string) *AppError {
	return &AppError{Code: ErrConflict, Message: msg}
}

func NewUnauthorized() *AppError {
	return &AppError{Code: ErrUnauthorized}
}

func NewNotFound() *AppError {
	return &AppError{Code: ErrNotFound}
}
