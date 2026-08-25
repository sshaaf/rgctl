package langfeatures

// LF-11 expression switch, LF-12 type switch, LF-13 select,
// LF-14 defer, LF-15 go.

func LfExprSwitch(n int) string {
	switch n {
	case 0:
		return "zero"
	case 1:
		return "one"
	default:
		return "other"
	}
}

func LfTypeSwitch(v any) string {
	switch v.(type) {
	case int:
		return "int"
	case string:
		return "string"
	default:
		return "other"
	}
}

func LfSelectLoop(ch <-chan int, done <-chan struct{}) int {
	select {
	case n := <-ch:
		return n
	case <-done:
		return -1
	}
}

func lfDeferredCleanup() {}

func LfWithDefer() {
	defer lfDeferredCleanup()
}

func lfSpawnedWork() {}

func LfSpawn() {
	go lfSpawnedWork()
}
