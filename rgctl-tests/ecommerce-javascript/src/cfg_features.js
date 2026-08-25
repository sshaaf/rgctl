/**
 * CFG feature probes for rgctl expected-facts (JavaScript lowering coverage).
 */

function cfgShortCircuit(a, b) {
  if (a && b) {
    return 1;
  }
  return 0;
}

function cfgForLoop(xs) {
  let total = 0;
  for (let i = 0; i < xs.length; i++) {
    total += xs[i];
  }
  return total;
}

function cfgTryCatch() {
  try {
    throw new Error("boom");
  } catch (e) {
    return 1;
  }
}

function cfgSwitch(x) {
  switch (x) {
    case 1:
      return 10;
    case 2:
      return 20;
    default:
      return 0;
  }
}

module.exports = { cfgShortCircuit, cfgForLoop, cfgTryCatch, cfgSwitch };
