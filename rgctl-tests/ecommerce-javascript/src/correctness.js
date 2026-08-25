/**
 * Intentional call graph for rgctl expected-facts checks.
 * Prefer direct function calls so extraction does not depend on DI.
 */

function correctnessLeaf() {
  return 42;
}

function correctnessMid() {
  return correctnessLeaf() + 1;
}

function correctnessRoot(flag) {
  const value = correctnessMid();
  if (flag) {
    return value * 2;
  }
  return value;
}

module.exports = { correctnessLeaf, correctnessMid, correctnessRoot };
