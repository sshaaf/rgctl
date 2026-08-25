package langfeatures

// LF-16 generics.

type LfBox[T any] struct {
	Value T
}

func LfIdentity[T any](v T) T {
	return v
}

func LfUseGeneric() int {
	return LfIdentity(42)
}
