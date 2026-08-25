package com.example.ecommerce.service;

import com.example.ecommerce.dto.InventoryDto;
import com.example.ecommerce.entity.Inventory;
import com.example.ecommerce.entity.Product;
import com.example.ecommerce.exception.ResourceNotFoundException;
import com.example.ecommerce.repository.InventoryRepository;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.util.List;

@Service
public class InventoryService {

    private final InventoryRepository inventoryRepository;
    private final ProductService productService;

    public InventoryService(InventoryRepository inventoryRepository, ProductService productService) {
        this.inventoryRepository = inventoryRepository;
        this.productService = productService;
    }

    @Transactional(readOnly = true)
    public List<InventoryDto> findAll() {
        return inventoryRepository.findAll().stream().map(this::toDto).toList();
    }

    @Transactional(readOnly = true)
    public InventoryDto findByProductId(Long productId) {
        return toDto(getByProductId(productId));
    }

    @Transactional
    public InventoryDto updateQuantity(Long productId, int quantity) {
        Inventory inventory = getByProductId(productId);
        inventory.setQuantity(quantity);
        return toDto(inventoryRepository.save(inventory));
    }

    private Inventory getByProductId(Long productId) {
        productService.getProduct(productId);
        return inventoryRepository.findByProductId(productId)
                .orElseThrow(() -> new ResourceNotFoundException("Inventory not found for product: " + productId));
    }

    private InventoryDto toDto(Inventory inventory) {
        Product product = inventory.getProduct();
        return new InventoryDto(inventory.getId(), product.getId(), product.getName(), inventory.getQuantity());
    }
}
