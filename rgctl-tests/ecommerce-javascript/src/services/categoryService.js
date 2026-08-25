const { v4: uuidv4 } = require('uuid');
const { getDb } = require('../db');
const categoryRepository = require('../repositories/categoryRepository');
const { AppError } = require('../utils/errors');
const { nowIso } = require('../utils/time');

function toResponse(category) {
  return { id: category.id, name: category.name, slug: category.slug };
}

function createCategory(req) {
  const db = getDb();
  const category = {
    id: uuidv4(),
    name: req.name,
    slug: req.slug,
    created_at: nowIso(),
  };
  categoryRepository.createCategory(db, category);
  return toResponse(category);
}

function listCategories() {
  const db = getDb();
  return categoryRepository.listCategories(db).map(toResponse);
}

function getCategory(id) {
  const db = getDb();
  const category = categoryRepository.findCategoryById(db, id);
  if (!category) {
    throw AppError.notFound();
  }
  return toResponse(category);
}

module.exports = { createCategory, listCategories, getCategory };
