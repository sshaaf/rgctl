package correctness

// Intentional call graph for rgctl expected-facts checks.

// CorrectnessLeaf is the leaf — no outbound application calls.
func CorrectnessLeaf() int {
	return 42
}

// CorrectnessMid calls CorrectnessLeaf.
func CorrectnessMid() int {
	return CorrectnessLeaf() + 1
}

// CorrectnessRoot calls CorrectnessMid and branches for a non-trivial CFG.
func CorrectnessRoot(flag bool) int {
	value := CorrectnessMid()
	if flag {
		return value * 2
	}
	return value
}
