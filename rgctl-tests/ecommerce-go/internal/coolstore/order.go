package coolstore

import "sync"

// OrderService stores CoolStore orders in memory.
type OrderService struct {
	mu     sync.Mutex
	seq    int64
	orders map[int64]*CoolstoreOrder
}

func NewOrderService() *OrderService {
	return &OrderService{
		seq:    1,
		orders: make(map[int64]*CoolstoreOrder),
	}
}

func (os *OrderService) Process(cart *ShoppingCart) *CoolstoreOrder {
	os.mu.Lock()
	orderID := os.seq
	os.seq++
	os.mu.Unlock()

	order := &CoolstoreOrder{
		OrderId:   orderID,
		CartId:    cart.CartId,
		CartTotal: cart.CartTotal,
		Items:     make([]*CoolstoreOrderItem, 0),
	}
	for _, sci := range cart.ShoppingCartItemList {
		if sci.Product != nil {
			order.Items = append(order.Items, &CoolstoreOrderItem{
				ProductId: sci.Product.ItemId,
				Quantity:  sci.Quantity,
				Price:     sci.Price,
			})
		}
	}
	os.mu.Lock()
	os.orders[order.OrderId] = order
	os.mu.Unlock()
	return order
}

func (os *OrderService) GetOrders() []*CoolstoreOrder {
	os.mu.Lock()
	defer os.mu.Unlock()
	out := make([]*CoolstoreOrder, 0, len(os.orders))
	for _, o := range os.orders {
		out = append(out, o)
	}
	return out
}

func (os *OrderService) GetOrderById(orderID int64) (*CoolstoreOrder, bool) {
	os.mu.Lock()
	defer os.mu.Unlock()
	o, ok := os.orders[orderID]
	return o, ok
}
