package coolstore

// CatalogProduct is a lightweight CoolStore catalog product (itemId keyed).
type CatalogProduct struct {
	ItemId string  `json:"itemId"`
	Name   string  `json:"name"`
	Desc   string  `json:"desc"`
	Price  float64 `json:"price"`
}

// ShoppingCartItem is a line item on a CoolStore cart.
type ShoppingCartItem struct {
	Price        float64         `json:"price"`
	Quantity     int             `json:"quantity"`
	PromoSavings float64         `json:"promoSavings"`
	Product      *CatalogProduct `json:"product"`
}

// ShoppingCart has mutable pricing totals (CPG field-write target).
type ShoppingCart struct {
	CartId               string              `json:"cartId"`
	CartItemTotal        float64             `json:"cartItemTotal"`
	CartItemPromoSavings float64             `json:"cartItemPromoSavings"`
	ShippingTotal        float64             `json:"shippingTotal"`
	ShippingPromoSavings float64             `json:"shippingPromoSavings"`
	CartTotal            float64             `json:"cartTotal"`
	ShoppingCartItemList []*ShoppingCartItem `json:"shoppingCartItemList"`
}

func NewShoppingCart(cartID string) *ShoppingCart {
	return &ShoppingCart{
		CartId:               cartID,
		ShoppingCartItemList: make([]*ShoppingCartItem, 0),
	}
}

func (sc *ShoppingCart) ResetShoppingCartItemList() {
	sc.ShoppingCartItemList = make([]*ShoppingCartItem, 0)
}

func (sc *ShoppingCart) AddShoppingCartItem(sci *ShoppingCartItem) {
	if sci != nil {
		sc.ShoppingCartItemList = append(sc.ShoppingCartItemList, sci)
	}
}

func (sc *ShoppingCart) RemoveShoppingCartItem(sci *ShoppingCartItem) bool {
	if sci == nil {
		return false
	}
	for i, item := range sc.ShoppingCartItemList {
		if item == sci {
			sc.ShoppingCartItemList = append(sc.ShoppingCartItemList[:i], sc.ShoppingCartItemList[i+1:]...)
			return true
		}
	}
	return false
}

// CoolstoreOrderItem is a line on a CoolStore order.
type CoolstoreOrderItem struct {
	ProductId string  `json:"productId"`
	Quantity  int     `json:"quantity"`
	Price     float64 `json:"price"`
}

// CoolstoreOrder is a checked-out CoolStore order.
type CoolstoreOrder struct {
	OrderId   int64                 `json:"orderId"`
	CartId    string                `json:"cartId"`
	CartTotal float64               `json:"cartTotal"`
	Items     []*CoolstoreOrderItem `json:"items"`
}
