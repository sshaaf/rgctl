package timeutil

import "time"

func NowISO() string {
	return time.Now().UTC().Format(time.RFC3339)
}
