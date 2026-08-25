package coolstore

// PromoService applies CoolStore item and shipping promotions.
type PromoService struct {
	percentOffByItem map[string]float64
}

func NewPromoService() *PromoService {
	return &PromoService{
		percentOffByItem: map[string]float64{
			"329299": 0.25,
		},
	}
}

func (ps *PromoService) ApplyCartItemPromotions(shoppingCart *ShoppingCart) {
	if shoppingCart == nil || len(shoppingCart.ShoppingCartItemList) == 0 {
		return
	}
	for _, sci := range shoppingCart.ShoppingCartItemList {
		if sci.Product == nil {
			continue
		}
		pct, ok := ps.percentOffByItem[sci.Product.ItemId]
		if !ok {
			continue
		}
		sci.PromoSavings = sci.Product.Price * pct * -1
		sci.Price = sci.Product.Price * (1 - pct)
	}
}

func (ps *PromoService) ApplyShippingPromotions(shoppingCart *ShoppingCart) {
	if shoppingCart == nil {
		return
	}
	if shoppingCart.CartItemTotal >= 75 {
		shoppingCart.ShippingPromoSavings = shoppingCart.ShippingTotal * -1
		shoppingCart.ShippingTotal = 0
	}
}
