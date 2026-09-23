/** Named-arrow fixture for extraction / blast-radius gates. */

function declaredAdd(a, b) {
  return a + b;
}

const arrowAdd = (a, b) => a + b;

const arrowHelper = () => 1;

const api = {
  fetchAll: async () => 0,
};

function callArrows() {
  return arrowAdd(1, 2) + arrowHelper();
}

[1].map((x) => x + 1);

module.exports = { declaredAdd, arrowAdd, arrowHelper, api, callArrows };
