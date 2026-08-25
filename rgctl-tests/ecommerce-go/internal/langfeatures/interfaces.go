package langfeatures

// LF-04 interface dispatch, LF-05 multiple implementations.

type LfRuntime interface {
	RunSandbox(name string) (string, error)
}

type LfRemoteRuntime struct{}

func (r *LfRemoteRuntime) RunSandbox(name string) (string, error) {
	return "remote:" + name, nil
}

type LfFakeRuntime struct{}

func (r *LfFakeRuntime) RunSandbox(name string) (string, error) {
	return "fake:" + name, nil
}

type LfRuntimeClient struct {
	runtime LfRuntime
}

func NewLfRuntimeClient(rt LfRuntime) *LfRuntimeClient {
	return &LfRuntimeClient{runtime: rt}
}

// Start dispatches through the LfRuntime interface.
func (c *LfRuntimeClient) Start(name string) (string, error) {
	return c.runtime.RunSandbox(name)
}
