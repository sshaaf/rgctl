package langfeatures

// Extra CFG probes beyond control.go (short-circuit + for continue).

func LfShortCircuit(a, b bool) int {
	if a && b {
		return 1
	}
	return 0
}

func LfForContinue(n int) int {
	total := 0
	for i := 0; i < n; i++ {
		if i == 1 {
			continue
		}
		total += i
	}
	return total
}
