package langfeatures

// LF-08 var declarations, LF-09 short var, LF-10 const/iota/alias.

type LfStatus int

const (
	LfStatusPending LfStatus = iota
	LfStatusActive
	LfStatusDone
)

type LfUserID string

func LfVarFlow(seed int) int {
	var x int
	x = seed
	var s string
	s = "ok"
	if s == "" {
		return 0
	}
	return x
}

func LfShortVarFlow(seed int) int {
	y := seed * 2
	return y
}

func LfStatusName(st LfStatus) string {
	switch st {
	case LfStatusPending:
		return "pending"
	case LfStatusActive:
		return "active"
	default:
		return "done"
	}
}
