package com.example.ecommerce.config;

import com.example.ecommerce.entity.Category;
import com.example.ecommerce.entity.Inventory;
import com.example.ecommerce.entity.Product;
import com.example.ecommerce.repository.CategoryRepository;
import com.example.ecommerce.repository.ProductRepository;
import org.springframework.boot.context.event.ApplicationReadyEvent;
import org.springframework.context.event.EventListener;
import org.springframework.stereotype.Component;
import org.springframework.transaction.annotation.Transactional;

import java.math.BigDecimal;

@Component
public class DataSeeder {

    private final CategoryRepository categoryRepository;
    private final ProductRepository productRepository;

    public DataSeeder(CategoryRepository categoryRepository, ProductRepository productRepository) {
        this.categoryRepository = categoryRepository;
        this.productRepository = productRepository;
    }

    @EventListener(ApplicationReadyEvent.class)
    @Transactional
    public void seedDemoData() {
        if (categoryRepository.count() > 0) {
            return;
        }

        Category electronics = new Category();
        electronics.setName("Electronics");
        electronics.setDescription("Gadgets and devices");
        electronics = categoryRepository.save(electronics);

        Product laptop = new Product();
        laptop.setName("Demo Laptop");
        laptop.setDescription("Lightweight laptop for everyday use");
        laptop.setPrice(new BigDecimal("999.99"));
        laptop.setCategory(electronics);
        Inventory laptopInventory = new Inventory();
        laptopInventory.setProduct(laptop);
        laptopInventory.setQuantity(25);
        laptop.setInventory(laptopInventory);
        productRepository.save(laptop);

        Product headphones = new Product();
        headphones.setName("Demo Headphones");
        headphones.setDescription("Wireless noise-cancelling headphones");
        headphones.setPrice(new BigDecimal("149.99"));
        headphones.setCategory(electronics);
        Inventory headphonesInventory = new Inventory();
        headphonesInventory.setProduct(headphones);
        headphonesInventory.setQuantity(100);
        headphones.setInventory(headphonesInventory);
        productRepository.save(headphones);
    }
}
