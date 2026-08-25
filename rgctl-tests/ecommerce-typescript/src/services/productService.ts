import { v4 as uuidv4 } from 'uuid';
import { getDb } from '../db';
import { Product } from '../models/product';
import * as categoryRepository from '../repositories/categoryRepository';
import * as productRepository from '../repositories/productRepository';
import { AppError } from '../utils/errors';
import { nowIso } from '../utils/time';

export interface CreateProductRequest {
  category_id: string;
  name: string;
  slug: string;
  description: string;
  price_cents: number;
  stock: number;
}

export interface ProductResponse {
  id: string;
  category_id: string;
  name: string;
  slug: string;
  description: string;
  price_cents: number;
  stock: number;
}

function toResponse(product: Product): ProductResponse {
  return {
    id: product.id,
    category_id: product.category_id,
    name: product.name,
    slug: product.slug,
    description: product.description,
    price_cents: product.price_cents,
    stock: product.stock,
  };
}

export function createProduct(req: CreateProductRequest): ProductResponse {
  const db = getDb();

  if (!categoryRepository.findCategoryById(db, req.category_id)) {
    throw AppError.badRequest('unknown category');
  }

  const product: Product = {
    id: uuidv4(),
    category_id: req.category_id,
    name: req.name,
    slug: req.slug,
    description: req.description,
    price_cents: req.price_cents,
    stock: req.stock,
    created_at: nowIso(),
  };

  productRepository.createProduct(db, product);
  return toResponse(product);
}

export function listProducts(): ProductResponse[] {
  const db = getDb();
  return productRepository.listProducts(db).map(toResponse);
}

export function getProduct(id: string): ProductResponse {
  const db = getDb();
  const product = productRepository.findProductById(db, id);
  if (!product) {
    throw AppError.notFound();
  }
  return toResponse(product);
}
