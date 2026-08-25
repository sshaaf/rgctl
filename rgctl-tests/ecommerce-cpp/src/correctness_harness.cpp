// Intentional call graph for rgctl expected-facts checks.
// Avoid intermediate locals that some extractors mis-label as callers.
// Definitions only in this .cpp (no header prototypes) to avoid ambiguous symbols.

namespace ecommerce {
namespace correctness {

int correctnessLeaf() { return 42; }

int correctnessMid() { return correctnessLeaf() + 1; }

int correctnessRoot(bool flag) {
  if (flag) {
    return correctnessMid() * 2;
  }
  return correctnessMid();
}

}  // namespace correctness
}  // namespace ecommerce
