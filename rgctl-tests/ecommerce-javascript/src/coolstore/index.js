module.exports = {
  ...require('./models/catalogProduct'),
  ...require('./models/shoppingCart'),
  ...require('./models/shoppingCartItem'),
  ...require('./models/coolstoreOrder'),
  ...require('./models/coolstoreOrderItem'),
  ...require('./services/coolstoreProductService'),
  ...require('./services/promoService'),
  ...require('./services/shippingService'),
  ...require('./services/coolstoreOrderService'),
  ...require('./services/shoppingCartService'),
  coolstoreRoutes: require('./routes'),
};
