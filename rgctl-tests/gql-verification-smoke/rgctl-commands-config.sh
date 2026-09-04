#!/usr/bin/env bash
# Per-language symbols and paths for rgctl command verification.
# Set RGCTL_CMD_ID before sourcing (e.g. python, java, php).

: "${RGCTL_CMD_ID:?RGCTL_CMD_ID must be set (e.g. python)}"

# shellcheck disable=SC2034
case "${RGCTL_CMD_ID}" in
  python)
    RGCTL_CMD_DISCOVER_EXTRA=(-l python -e .venv,__pycache__ --with-cfg)
    RGCTL_CMD_BLAST_PRIMARY='app/services/order.py::checkout'
    RGCTL_CMD_BLAST_COOLSTORE='price_shopping_cart'
    RGCTL_CMD_INSPECT_FN='checkout'
    RGCTL_CMD_CPG_TYPE='ShoppingCart'
    RGCTL_CMD_CPG_MIN_LINES=1
    RGCTL_CMD_EXPORT_QUERY='name:checkout'
    RGCTL_CMD_SEMANTIC_QUERY='shopping cart checkout'
    RGCTL_CMD_SLICE_FILE='app/services/order.py'
    RGCTL_CMD_SLICE_LINE=38
    RGCTL_CMD_SLICE_VAR='total_cents'
    RGCTL_CMD_SLICE_FN='checkout'
    ;;
  javascript)
    RGCTL_CMD_DISCOVER_EXTRA=(-l javascript -e node_modules --with-cfg)
    RGCTL_CMD_BLAST_PRIMARY='clearCart'
    RGCTL_CMD_BLAST_COOLSTORE='priceShoppingCart'
    RGCTL_CMD_INSPECT_FN='checkout'
    RGCTL_CMD_CPG_TYPE='ShoppingCart'
    RGCTL_CMD_CPG_MIN_LINES=0
    RGCTL_CMD_EXPORT_QUERY='name:clearCart'
    RGCTL_CMD_SEMANTIC_QUERY='shopping cart checkout'
    RGCTL_CMD_SLICE_FILE=''
    ;;
  typescript)
    RGCTL_CMD_DISCOVER_EXTRA=(-l typescript -e node_modules,dist --with-cfg)
    RGCTL_CMD_BLAST_PRIMARY='clearCart'
    RGCTL_CMD_BLAST_COOLSTORE='priceShoppingCart'
    RGCTL_CMD_INSPECT_FN='checkout'
    RGCTL_CMD_CPG_TYPE='ShoppingCart'
    RGCTL_CMD_CPG_MIN_LINES=0
    RGCTL_CMD_EXPORT_QUERY='name:clearCart'
    RGCTL_CMD_SEMANTIC_QUERY='shopping cart checkout'
    RGCTL_CMD_SLICE_FILE=''
    ;;
  csharp)
    RGCTL_CMD_DISCOVER_EXTRA=(-l csharp -e bin,obj,data --with-cfg)
    RGCTL_CMD_BLAST_PRIMARY='ClearCartAsync'
    RGCTL_CMD_BLAST_COOLSTORE='PriceShoppingCart'
    RGCTL_CMD_INSPECT_FN='CheckoutAsync'
    RGCTL_CMD_CPG_TYPE='ShoppingCart'
    RGCTL_CMD_CPG_MIN_LINES=1
    RGCTL_CMD_EXPORT_QUERY='name:ClearCartAsync'
    RGCTL_CMD_SEMANTIC_QUERY='shopping cart checkout'
    RGCTL_CMD_SLICE_FILE='src/Ecommerce/Services/OrderService.cs'
    RGCTL_CMD_SLICE_LINE=16
    RGCTL_CMD_SLICE_VAR='total'
    RGCTL_CMD_SLICE_FN='CheckoutAsync'
    ;;
  java)
    RGCTL_CMD_DISCOVER_EXTRA=(-l java -e target,data --with-cfg)
    RGCTL_CMD_BLAST_PRIMARY='CartService::clearCart'
    RGCTL_CMD_BLAST_COOLSTORE='ShoppingCartService::priceShoppingCart'
    RGCTL_CMD_INSPECT_FN='checkout'
    RGCTL_CMD_CPG_TYPE='ShoppingCart'
    RGCTL_CMD_CPG_MIN_LINES=1
    RGCTL_CMD_EXPORT_QUERY='name:clearCart'
    RGCTL_CMD_SEMANTIC_QUERY='shopping cart checkout'
    RGCTL_CMD_SLICE_FILE='src/main/java/com/example/ecommerce/service/CartService.java'
    RGCTL_CMD_SLICE_LINE=53
    RGCTL_CMD_SLICE_VAR='item'
    RGCTL_CMD_SLICE_FN='addItem'
    ;;
  go)
    RGCTL_CMD_DISCOVER_EXTRA=(-l go -e vendor --with-cfg)
    RGCTL_CMD_BLAST_PRIMARY='internal/service/order.go::Checkout'
    RGCTL_CMD_BLAST_COOLSTORE='PriceShoppingCart'
    RGCTL_CMD_INSPECT_FN='Checkout'
    RGCTL_CMD_CPG_TYPE='ShoppingCart'
    RGCTL_CMD_CPG_MIN_LINES=1
    RGCTL_CMD_EXPORT_QUERY='name:Checkout'
    RGCTL_CMD_SEMANTIC_QUERY='shopping cart checkout'
    RGCTL_CMD_SLICE_FILE='internal/service/order.go'
    RGCTL_CMD_SLICE_LINE=16
    RGCTL_CMD_SLICE_VAR='total'
    RGCTL_CMD_SLICE_FN='Checkout'
    ;;
  rust)
    RGCTL_CMD_DISCOVER_EXTRA=(-l rust -e target --with-cfg)
    RGCTL_CMD_BLAST_PRIMARY='src/services/order.rs::checkout'
    RGCTL_CMD_BLAST_COOLSTORE='price_shopping_cart'
    RGCTL_CMD_INSPECT_FN='checkout'
    RGCTL_CMD_CPG_TYPE='ShoppingCart'
    RGCTL_CMD_CPG_MIN_LINES=1
    RGCTL_CMD_EXPORT_QUERY='name:checkout'
    RGCTL_CMD_SEMANTIC_QUERY='shopping cart checkout'
    RGCTL_CMD_SLICE_FILE='src/services/order.rs'
    RGCTL_CMD_SLICE_LINE=16
    RGCTL_CMD_SLICE_VAR='total'
    RGCTL_CMD_SLICE_FN='checkout'
    ;;
  c)
    RGCTL_CMD_DISCOVER_EXTRA=(-l c -e build,cmake-build-debug,.rgctl --with-cfg)
    RGCTL_CMD_BLAST_PRIMARY='src/coolstore/services/shopping_cart_service.c::price_shopping_cart'
    RGCTL_CMD_BLAST_COOLSTORE='src/coolstore/services/shopping_cart_service.c::price_shopping_cart'
    RGCTL_CMD_INSPECT_FN='price_shopping_cart'
    RGCTL_CMD_CPG_TYPE='ShoppingCart'
    RGCTL_CMD_CPG_MIN_LINES=0
    RGCTL_CMD_EXPORT_QUERY='name:price_shopping_cart'
    RGCTL_CMD_SEMANTIC_QUERY='shopping cart pricing'
    RGCTL_CMD_SLICE_FILE='src/coolstore/services/shopping_cart_service.c'
    RGCTL_CMD_SLICE_LINE=16
    RGCTL_CMD_SLICE_VAR='cart'
    RGCTL_CMD_SLICE_FN='price_shopping_cart'
    ;;
  cpp)
    RGCTL_CMD_DISCOVER_EXTRA=(-l cpp -e build,cmake-build-debug,.rgctl --with-cfg)
    RGCTL_CMD_BLAST_PRIMARY='src/coolstore/services/shopping_cart_service.cpp::priceShoppingCart'
    RGCTL_CMD_BLAST_COOLSTORE='src/coolstore/services/shopping_cart_service.cpp::priceShoppingCart'
    RGCTL_CMD_INSPECT_FN='priceShoppingCart'
    RGCTL_CMD_CPG_TYPE='ShoppingCart'
    RGCTL_CMD_CPG_MIN_LINES=1
    RGCTL_CMD_EXPORT_QUERY='name:priceShoppingCart'
    RGCTL_CMD_SEMANTIC_QUERY='shopping cart pricing'
    RGCTL_CMD_SLICE_FILE='src/coolstore/services/shopping_cart_service.cpp'
    RGCTL_CMD_SLICE_LINE=16
    RGCTL_CMD_SLICE_VAR='cart'
    RGCTL_CMD_SLICE_FN='priceShoppingCart'
    ;;
  php)
    RGCTL_CMD_DISCOVER_EXTRA=(-l php --with-cfg --with-taint)
    RGCTL_CMD_BLAST_PRIMARY='src/Service/AuthService.php::login'
    RGCTL_CMD_BLAST_COOLSTORE='src/Service/AuthService.php::processOrder'
    RGCTL_CMD_INSPECT_FN='login'
    RGCTL_CMD_CPG_TYPE='OrderDTO'
    RGCTL_CMD_CPG_MIN_LINES=0
    RGCTL_CMD_EXPORT_QUERY='name:login'
    RGCTL_CMD_SEMANTIC_QUERY='user login authentication'
    RGCTL_CMD_SLICE_FILE='src/Service/AuthService.php'
    RGCTL_CMD_SLICE_LINE=35
    RGCTL_CMD_SLICE_VAR='order'
    RGCTL_CMD_SLICE_FN='processOrder'
    ;;
  *)
    echo "error: unknown RGCTL_CMD_ID=${RGCTL_CMD_ID}" >&2
    exit 1
    ;;
esac

RGCTL_CMD_POLICY="${RGCTL_TESTS}/rgctl-policy.json"
