package langfeatures

// LF-06 struct embedding, LF-07 promoted / base method call.

type LfBase struct {
	Label string
}

func (b *LfBase) BaseMethod() string {
	return b.Label
}

type LfDerived struct {
	LfBase
	Extra int
}

func (d *LfDerived) UseBase() string {
	return d.BaseMethod()
}
