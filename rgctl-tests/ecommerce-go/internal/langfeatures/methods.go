package langfeatures

// LF-02 same-type method calls, LF-03 cross-type same-name resolution,
// LF-18 NewT constructor, LF-20 multi-value return.

type LfCart struct {
	Items int
}

func NewLfCart(items int) *LfCart {
	return &LfCart{Items: items}
}

func (c *LfCart) validate() bool {
	return c.Items >= 0
}

func (c *LfCart) Checkout() bool {
	if !c.validate() {
		return false
	}
	return true
}

func (c *LfCart) Totals() (int, error) {
	return c.Items, nil
}

// Same method name on two types — resolution must not collapse.
type LfAlphaStore struct{}

func (s *LfAlphaStore) ListItems() int { return 10 }

type LfBetaStore struct{}

func (s *LfBetaStore) ListItems() int { return 20 }

type LfOrchestrator struct {
	alpha *LfAlphaStore
	beta  *LfBetaStore
}

func NewLfOrchestrator(a *LfAlphaStore, b *LfBetaStore) *LfOrchestrator {
	return &LfOrchestrator{alpha: a, beta: b}
}

// Run must CALL LfBetaStore.ListItems (not Alpha).
func (o *LfOrchestrator) Run() int {
	return o.beta.ListItems()
}
