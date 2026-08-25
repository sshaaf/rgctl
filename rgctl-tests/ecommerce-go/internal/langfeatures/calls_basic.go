package langfeatures

// LF-01: package-level function call.

func LfPkgCallee() int { return 1 }

func LfPkgCaller() int { return LfPkgCallee() + 1 }
