"""CoolStore dual-API package (`/services/products|cart|orders`)."""

from app.coolstore.routers import cart_router, orders_router, products_router

__all__ = ["cart_router", "orders_router", "products_router"]
