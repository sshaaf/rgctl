package coolstore

import "sync"

// ShoppingCartService manages in-memory carts and pricing mutations.
type ShoppingCartService struct {
	productService  *ProductService
	promoService    *PromoService
	shippingService *ShippingService
	orderService    *OrderService
	mu              sync.Mutex
	carts           map[string]*ShoppingCart
}

func NewShoppingCartService(
	productService *ProductService,
	promoService *PromoService,
	shippingService *ShippingService,
	orderService *OrderService,
) *ShoppingCartService {
	return &ShoppingCartService{
		productService:  productService,
		promoService:    promoService,
		shippingService: shippingService,
		orderService:    orderService,
		carts:           make(map[string]*ShoppingCart),
	}
}

func (s *ShoppingCartService) GetShoppingCart(cartID string) *ShoppingCart {
	s.mu.Lock()
	defer s.mu.Unlock()
	cart, ok := s.carts[cartID]
	if !ok {
		cart = NewShoppingCart(cartID)
		s.carts[cartID] = cart
	}
	return cart
}

func (s *ShoppingCartService) GetProduct(itemID string) *CatalogProduct {
	return s.productService.GetProductByItemId(itemID)
}

func (s *ShoppingCartService) CheckOutShoppingCart(cartID string) *ShoppingCart {
	cart := s.GetShoppingCart(cartID)
	s.PriceShoppingCart(cart)
	s.orderService.Process(cart)
	cart.ResetShoppingCartItemList()
	s.PriceShoppingCart(cart)
	return cart
}

// PriceShoppingCart mutates ShoppingCart totals — primary CPG field-write site.
func (s *ShoppingCartService) PriceShoppingCart(sc *ShoppingCart) {
	if sc == nil {
		return
	}
	s.initShoppingCartForPricing(sc)

	if len(sc.ShoppingCartItemList) > 0 {
		s.promoService.ApplyCartItemPromotions(sc)

		for _, sci := range sc.ShoppingCartItemList {
			sc.CartItemPromoSavings = sc.CartItemPromoSavings + sci.PromoSavings*float64(sci.Quantity)
			sc.CartItemTotal = sc.CartItemTotal + sci.Price*float64(sci.Quantity)
		}

		sc.ShippingTotal = s.shippingService.CalculateShipping(sc)
		if sc.CartItemTotal >= 25 {
			sc.ShippingTotal = sc.ShippingTotal + s.shippingService.CalculateShippingInsurance(sc)
		}
	}

	s.promoService.ApplyShippingPromotions(sc)
	sc.CartTotal = sc.CartItemTotal + sc.ShippingTotal
}

func (s *ShoppingCartService) initShoppingCartForPricing(sc *ShoppingCart) {
	sc.CartItemTotal = 0
	sc.CartItemPromoSavings = 0
	sc.ShippingTotal = 0
	sc.ShippingPromoSavings = 0
	sc.CartTotal = 0

	for _, sci := range sc.ShoppingCartItemList {
		if sci.Product != nil {
			p := s.GetProduct(sci.Product.ItemId)
			if p != nil {
				sci.Product = p
				sci.Price = p.Price
			}
		}
		sci.PromoSavings = 0
	}
}

func (s *ShoppingCartService) DedupeCartItems(cartItems []*ShoppingCartItem) []*ShoppingCartItem {
	quantityMap := make(map[string]int)
	for _, sci := range cartItems {
		if sci.Product == nil {
			continue
		}
		quantityMap[sci.Product.ItemId] += sci.Quantity
	}
	result := make([]*ShoppingCartItem, 0, len(quantityMap))
	for itemID, quantity := range quantityMap {
		p := s.GetProduct(itemID)
		if p == nil {
			continue
		}
		result = append(result, &ShoppingCartItem{
			Quantity: quantity,
			Price:    p.Price,
			Product:  p,
		})
	}
	return result
}
