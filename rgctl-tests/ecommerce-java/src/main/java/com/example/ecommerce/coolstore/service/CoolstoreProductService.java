package com.example.ecommerce.coolstore.service;

import com.example.ecommerce.coolstore.model.CatalogProduct;
import com.example.ecommerce.coolstore.model.ShoppingCart;
import com.example.ecommerce.coolstore.model.ShoppingCartItem;
import org.springframework.stereotype.Service;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

@Service
public class CoolstoreProductService {

    private final Map<String, CatalogProduct> catalog = new ConcurrentHashMap<>();

    public CoolstoreProductService() {
        seed("329299", "Red Fedora", "Official Red Hat Fedora", 34.99);
        seed("329199", "Forge Laptop Sticker", "JBoss Community sticker", 8.50);
        seed("165613", "Solid Performance Polo", "Moisture-wicking polo", 17.80);
        seed("165614", "Ogios T-shirt", "CoolStore tee", 11.50);
        seed("165954", "Quarkus Stickers", "Pack of stickers", 9.99);
    }

    private void seed(String id, String name, String desc, double price) {
        catalog.put(id, new CatalogProduct(id, name, desc, price));
    }

    public List<CatalogProduct> getProducts() {
        return List.copyOf(catalog.values());
    }

    public CatalogProduct getProductByItemId(String itemId) {
        return catalog.get(itemId);
    }

    public Map<String, CatalogProduct> asMap() {
        return new HashMap<>(catalog);
    }
}
