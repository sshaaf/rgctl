// CFG feature probes for rgctl expected-facts (C++ lowering coverage).

namespace ecommerce {
namespace correctness {

int cfgShortCircuit(bool a, bool b) {
  if (a && b) {
    return 1;
  }
  return 0;
}

int cfgRangeFor() {
  int xs[] = {1, 2, 3};
  int total = 0;
  for (int v : xs) {
    total += v;
  }
  return total;
}

int cfgTryCatch(bool boom) {
  try {
    if (boom) {
      throw 1;
    }
    return 0;
  } catch (int) {
    return 1;
  }
}

int cfgIfInit(int n) {
  if (int x = n; x > 0) {
    return x;
  }
  return 0;
}

int cfgTernary(int x) { return x > 0 ? x : -x; }

}  // namespace correctness
}  // namespace ecommerce
