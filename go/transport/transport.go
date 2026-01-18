package transport

type Transport interface {
	Send(data []byte) error
	Recv() ([]byte, error)
	Close() error
}
