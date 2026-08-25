package com.example.ecommerce.coolstore.rest;

import com.example.ecommerce.coolstore.model.CatalogProduct;
import com.example.ecommerce.coolstore.service.CoolstoreProductService;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

import java.util.List;

@RestController
@RequestMapping("/services/products")
public class ProductEndpoint {

    private final CoolstoreProductService productService;

    public ProductEndpoint(CoolstoreProductService productService) {
        this.productService = productService;
    }

    @GetMapping
    public List<CatalogProduct> listAll() {
        return productService.getProducts();
    }

    @GetMapping("/{itemId}")
    public CatalogProduct getProduct(@PathVariable String itemId) {
        return productService.getProductByItemId(itemId);
    }
}
