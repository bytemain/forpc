package mangos

import (
	"encoding/binary"
	"errors"
	"sync"
	"sync/atomic"
	"time"

	"go.nanomsg.org/mangos/v3"
	"go.nanomsg.org/mangos/v3/protocol/xrep"
	"go.nanomsg.org/mangos/v3/protocol/xreq"
	_ "go.nanomsg.org/mangos/v3/transport/inproc"
	_ "go.nanomsg.org/mangos/v3/transport/ipc"
	_ "go.nanomsg.org/mangos/v3/transport/tcp"
)

type Dealer struct {
	sock          mangos.Socket
	nextRequestID atomic.Uint32
}

func Dial(url string) (*Dealer, error) {
	s, err := xreq.NewSocket()
	if err != nil {
		return nil, err
	}
	if err := s.Dial(url); err != nil {
		_ = s.Close()
		return nil, err
	}
	d := &Dealer{sock: s}
	d.nextRequestID.Store(0x80000000)
	return d, nil
}

func DialWithRetry(url string, maxRetries uint32, retryDelay time.Duration) (*Dealer, error) {
	attempts := maxRetries + 1
	var lastErr error
	for i := uint32(0); i < attempts; i++ {
		if i > 0 {
			time.Sleep(retryDelay)
		}
		d, err := Dial(url)
		if err == nil {
			return d, nil
		}
		lastErr = err
	}
	if lastErr == nil {
		lastErr = errors.New("dial failed")
	}
	return nil, lastErr
}

func (d *Dealer) Send(body []byte) (uint32, error) {
	reqID := d.nextRequestID.Add(1) | 0x80000000
	msg := mangos.NewMessage(len(body))
	msg.Body = append(msg.Body, body...)
	var hdr [4]byte
	binary.BigEndian.PutUint32(hdr[:], reqID)
	msg.Header = append(msg.Header, hdr[:]...)
	return reqID, d.sock.SendMsg(msg)
}

func (d *Dealer) Recv() (*mangos.Message, error) {
	return d.sock.RecvMsg()
}

func (d *Dealer) Close() error {
	return d.sock.Close()
}

type Router struct {
	sock mangos.Socket
}

func Listen(url string) (*Router, error) {
	s, err := xrep.NewSocket()
	if err != nil {
		return nil, err
	}
	if err := s.Listen(url); err != nil {
		_ = s.Close()
		return nil, err
	}
	return &Router{sock: s}, nil
}

func (r *Router) Recv() (*mangos.Message, error) {
	return r.sock.RecvMsg()
}

func (r *Router) Send(msg *mangos.Message) error {
	return r.sock.SendMsg(msg)
}

func (r *Router) Clone() *Router {
	return &Router{sock: r.sock}
}

func (r *Router) Close() error {
	return r.sock.Close()
}

type InboundFrame struct {
	Header []byte
	Body   []byte
}

type ClientTransport struct {
	dealer    *Dealer
	sendCh    chan []byte
	recvCh    chan []byte
	done      chan struct{}
	closeOnce sync.Once
}

func NewClientTransport(url string) (*ClientTransport, error) {
	return NewClientTransportWithRetry(url, 10, 100*time.Millisecond)
}

func NewClientTransportWithRetry(url string, maxRetries uint32, retryDelay time.Duration) (*ClientTransport, error) {
	dealer, err := DialWithRetry(url, maxRetries, retryDelay)
	if err != nil {
		return nil, err
	}

	t := &ClientTransport{
		dealer: dealer,
		sendCh: make(chan []byte, 256),
		recvCh: make(chan []byte, 256),
		done:   make(chan struct{}),
	}

	go func() {
		defer close(t.done)
		defer close(t.recvCh)

		var wg sync.WaitGroup
		wg.Add(2)

		go func() {
			defer wg.Done()
			for body := range t.sendCh {
				if _, err := t.dealer.Send(body); err != nil {
					_ = t.dealer.Close()
					return
				}
			}
		}()

		go func() {
			defer wg.Done()
			for {
				msg, err := t.dealer.Recv()
				if err != nil {
					return
				}
				b := make([]byte, len(msg.Body))
				copy(b, msg.Body)
				msg.Free()
				select {
				case t.recvCh <- b:
				default:
					_ = t.dealer.Close()
					return
				}
			}
		}()

		wg.Wait()
		_ = t.dealer.Close()
	}()

	return t, nil
}

func (t *ClientTransport) Send(data []byte) error {
	select {
	case t.sendCh <- data:
		return nil
	case <-t.done:
		return errors.New("transport closed")
	}
}

func (t *ClientTransport) Recv() ([]byte, error) {
	b, ok := <-t.recvCh
	if !ok {
		return nil, errors.New("transport closed")
	}
	return b, nil
}

func (t *ClientTransport) Close() error {
	var closeErr error
	t.closeOnce.Do(func() {
		close(t.sendCh)
		closeErr = t.dealer.Close()
	})
	select {
	case <-t.done:
		return closeErr
	case <-time.After(1 * time.Second):
		return errors.New("close timeout")
	}
}

type ServerTransport struct {
	sendCh  chan *mangos.Message
	recvCh  chan InboundFrame
	routeMu sync.Mutex
	routing map[uint32][]byte
	done    chan struct{}
}

func NewServerTransport(router *Router, recv <-chan InboundFrame) *ServerTransport {
	t := &ServerTransport{
		sendCh:  make(chan *mangos.Message, 256),
		recvCh:  make(chan InboundFrame, 256),
		routing: make(map[uint32][]byte),
		done:    make(chan struct{}),
	}

	go func() {
		defer close(t.done)
		for msg := range t.sendCh {
			if err := router.Send(msg); err != nil {
				return
			}
		}
	}()

	go func() {
		for f := range recv {
			t.recvCh <- f
		}
		close(t.recvCh)
	}()

	return t
}

func parseStreamID(b []byte) (uint32, bool) {
	if len(b) < 4 {
		return 0, false
	}
	return binary.BigEndian.Uint32(b[:4]), true
}

func (t *ServerTransport) Send(data []byte) error {
	streamID, ok := parseStreamID(data)
	if !ok {
		return errors.New("packet too short")
	}

	t.routeMu.Lock()
	hdr, ok := t.routing[streamID]
	t.routeMu.Unlock()
	if !ok {
		return errors.New("missing routing header for stream")
	}

	msg := mangos.NewMessage(len(data))
	msg.Body = append(msg.Body, data...)
	msg.Header = append(msg.Header, hdr...)
	return t.sendMsg(msg)
}

func (t *ServerTransport) sendMsg(msg *mangos.Message) error {
	select {
	case t.sendCh <- msg:
		return nil
	case <-t.done:
		return errors.New("transport closed")
	}
}

func (t *ServerTransport) Recv() ([]byte, error) {
	f, ok := <-t.recvCh
	if !ok {
		return nil, errors.New("transport closed")
	}
	if streamID, ok := parseStreamID(f.Body); ok {
		t.routeMu.Lock()
		h := make([]byte, len(f.Header))
		copy(h, f.Header)
		t.routing[streamID] = h
		t.routeMu.Unlock()
	}
	b := make([]byte, len(f.Body))
	copy(b, f.Body)
	return b, nil
}

func (t *ServerTransport) Close() error {
	select {
	case <-t.done:
		return nil
	default:
		close(t.sendCh)
		<-t.done
		return nil
	}
}
