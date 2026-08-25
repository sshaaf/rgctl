import { v4 as uuidv4 } from 'uuid';
import { getDb } from '../db';
import { Category } from '../models/category';
import * as categoryRepository from '../repositories/categoryRepository';
import { AppError } from '../utils/errors';
import { nowIso } from '../utils/time';

export interface CreateCategoryRequest {
  name: string;
  slug: string;
}

export interface CategoryResponse {
  id: string;
  name: string;
  slug: string;
}

function toResponse(category: Category): CategoryResponse {
  return { id: category.id, name: category.name, slug: category.slug };
}

export function createCategory(req: CreateCategoryRequest): CategoryResponse {
  const db = getDb();
  const category: Category = {
    id: uuidv4(),
    name: req.name,
    slug: req.slug,
    created_at: nowIso(),
  };
  categoryRepository.createCategory(db, category);
  return toResponse(category);
}

export function listCategories(): CategoryResponse[] {
  const db = getDb();
  return categoryRepository.listCategories(db).map(toResponse);
}

export function getCategory(id: string): CategoryResponse {
  const db = getDb();
  const category = categoryRepository.findCategoryById(db, id);
  if (!category) {
    throw AppError.notFound();
  }
  return toResponse(category);
}
