package langfeatures

// LF-19 receiver field write, LF-21 struct tags.

type LfOrderDTO struct {
	Status string `json:"status"`
	Total  int    `json:"total"`
}

func (o *LfOrderDTO) MarkProcessed() {
	o.Status = "PROCESSED"
}

func NewLfOrderDTO() *LfOrderDTO {
	return &LfOrderDTO{Status: "NEW", Total: 0}
}
