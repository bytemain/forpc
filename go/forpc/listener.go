package forpc

import (
	"errors"
	"sync"

	mangostransport "github.com/bytemain/forpc/go/transport/mangos"
)

type RpcListener struct {
	router   *mangostransport.Router
	acceptCh chan *RpcPeer
	done     chan struct{}

	mu    sync.Mutex
	peers map[string]chan mangostransport.InboundFrame
}

func Bind(url string) (*RpcListener, error) {
	router, err := mangostransport.Listen(url)
	if err != nil {
		return nil, err
	}

	l := &RpcListener{
		router:   router,
		acceptCh: make(chan *RpcPeer, 100),
		done:     make(chan struct{}),
		peers:    make(map[string]chan mangostransport.InboundFrame),
	}

	go l.loop()

	return l, nil
}

func (l *RpcListener) Close() error {
	select {
	case <-l.done:
		return nil
	default:
		close(l.done)
		_ = l.router.Close()
		l.mu.Lock()
		for _, ch := range l.peers {
			close(ch)
		}
		l.peers = map[string]chan mangostransport.InboundFrame{}
		l.mu.Unlock()
		close(l.acceptCh)
		return nil
	}
}

func (l *RpcListener) Accept() (*RpcPeer, error) {
	p, ok := <-l.acceptCh
	if !ok {
		return nil, errors.New("listener closed")
	}
	return p, nil
}

func (l *RpcListener) loop() {
	for {
		select {
		case <-l.done:
			return
		default:
		}

		msg, err := l.router.Recv()
		if err != nil {
			_ = l.Close()
			return
		}
		if len(msg.Header) < 4 {
			msg.Free()
			continue
		}

		idKey := string(msg.Header[:4])
		hdr := make([]byte, len(msg.Header))
		copy(hdr, msg.Header)
		body := make([]byte, len(msg.Body))
		copy(body, msg.Body)
		msg.Free()

		var ch chan mangostransport.InboundFrame
		var peer *RpcPeer
		l.mu.Lock()
		ch = l.peers[idKey]
		if ch == nil {
			ch = make(chan mangostransport.InboundFrame, 100)
			l.peers[idKey] = ch
			t := mangostransport.NewServerTransport(l.router.Clone(), ch)
			peer = NewPeer(t, false)
		}
		l.mu.Unlock()

		if peer != nil {
			select {
			case l.acceptCh <- peer:
			case <-l.done:
				return
			}
		}

		select {
		case ch <- mangostransport.InboundFrame{Header: hdr, Body: body}:
		case <-l.done:
			return
		}
	}
}
