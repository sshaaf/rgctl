from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from app.config import settings
from app.coolstore import cart_router, orders_router, products_router
from app.database import SessionLocal, init_db
from app.routers import auth, cart, categories, health, orders, products, reviews, users
from app.utils.seed import seed_demo_data


@asynccontextmanager
async def lifespan(_app: FastAPI):
    init_db()
    db = SessionLocal()
    try:
        seed_demo_data(db)
    finally:
        db.close()
    yield


app = FastAPI(title="E-Commerce API", version="1.0.0", lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(health.router)
app.include_router(auth.router)
app.include_router(users.router)
app.include_router(categories.router)
app.include_router(products.router)
app.include_router(cart.router)
app.include_router(orders.router)
app.include_router(reviews.router)

# CoolStore dual-API (/services/*) — unauthenticated in-memory store
app.include_router(products_router)
app.include_router(cart_router)
app.include_router(orders_router)


if __name__ == "__main__":
    import uvicorn

    uvicorn.run("app.main:app", host=settings.bind_host, port=settings.bind_port, reload=True)
