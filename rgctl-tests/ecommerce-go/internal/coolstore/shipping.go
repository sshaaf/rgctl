package coolstore

import "math"

// ShippingService computes CoolStore shipping tiers and insurance.
type ShippingService struct{}

func NewShippingService() *ShippingService {
	return &ShippingService{}
}

func (ss *ShippingService) CalculateShipping(sc *ShoppingCart) float64 {
	if sc == nil {
		return 0
	}
	total := sc.CartItemTotal
	switch {
	case total >= 0 && total < 25:
		return 2.99
	case total >= 25 && total < 50:
		return 4.99
	case total >= 50 && total < 75:
		return 6.99
	case total >= 75 && total < 100:
		return 8.99
	case total >= 100:
		return 10.99
	default:
		return 0
	}
}

func (ss *ShippingService) CalculateShippingInsurance(sc *ShoppingCart) float64 {
	if sc == nil {
		return 0
	}
	total := sc.CartItemTotal
	switch {
	case total >= 25 && total < 100:
		return round2(total * 0.02)
	case total >= 100:
		return round2(total * 0.015)
	default:
		return 0
	}
}

func round2(v float64) float64 {
	return math.Round(v*100) / 100
}
