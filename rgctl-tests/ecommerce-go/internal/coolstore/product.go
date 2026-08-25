package coolstore

import "sync"

// ProductService is an in-memory CoolStore catalog.
type ProductService struct {
	mu      sync.RWMutex
	catalog map[string]*CatalogProduct
}

func NewProductService() *ProductService {
	ps := &ProductService{catalog: make(map[string]*CatalogProduct)}
	ps.seed("329299", "Red Fedora", "Official Red Hat Fedora", 34.99)
	ps.seed("329199", "Forge Laptop Sticker", "JBoss Community sticker", 8.50)
	ps.seed("165613", "Solid Performance Polo", "Moisture-wicking polo", 17.80)
	ps.seed("165614", "Ogios T-shirt", "CoolStore tee", 11.50)
	ps.seed("165954", "Quarkus Stickers", "Pack of stickers", 9.99)
	return ps
}

func (ps *ProductService) seed(id, name, desc string, price float64) {
	ps.catalog[id] = &CatalogProduct{ItemId: id, Name: name, Desc: desc, Price: price}
}

func (ps *ProductService) GetProducts() []*CatalogProduct {
	ps.mu.RLock()
	defer ps.mu.RUnlock()
	out := make([]*CatalogProduct, 0, len(ps.catalog))
	for _, p := range ps.catalog {
		out = append(out, p)
	}
	return out
}

func (ps *ProductService) GetProductByItemId(itemID string) *CatalogProduct {
	ps.mu.RLock()
	defer ps.mu.RUnlock()
	return ps.catalog[itemID]
}
