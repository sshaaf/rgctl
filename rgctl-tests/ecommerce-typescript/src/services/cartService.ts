import { getDb } from '../db';
import { CartItem } from '../models/cart';
import * as cartRepository from '../repositories/cartRepository';
import * as productRepository from '../repositories/productRepository';
import { AppError } from '../utils/errors';

export interface AddCartItemRequest {
  product_id: string;
  quantity: number;
}

export interface CartItemResponse {
  product_id: string;
  quantity: number;
}

export function listCart(userId: string): CartItemResponse[] {
  const db = getDb();
  return cartRepository.listCartItems(db, userId).map((item) => ({
    product_id: item.product_id,
    quantity: item.quantity,
  }));
}

export function addCartItem(userId: string, req: AddCartItemRequest): CartItemResponse {
  const db = getDb();

  if (req.quantity <= 0) {
    throw AppError.badRequest('quantity must be positive');
  }

  if (!productRepository.findProductById(db, req.product_id)) {
    throw AppError.notFound();
  }

  const item: CartItem = {
    user_id: userId,
    product_id: req.product_id,
    quantity: req.quantity,
  };

  cartRepository.upsertCartItem(db, item);
  return { product_id: req.product_id, quantity: req.quantity };
}

export function removeCartItem(userId: string, productId: string): void {
  const db = getDb();
  cartRepository.removeCartItem(db, userId, productId);
}
