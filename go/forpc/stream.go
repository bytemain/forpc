package forpc

import (
	"errors"
	"sync"
)

type BidiStream[Req any, Resp any] struct {
	streamID uint32
	peer     *RpcPeer
	recvCh   <-chan Packet
	mu       sync.Mutex
	closed   bool
}

func (s *BidiStream[Req, Resp]) Send(msg *Req) error {
	payload, err := s.peer.userMarshal(msg)
	if err != nil {
		return err
	}
	return s.peer.sendPacket(Packet{StreamID: s.streamID, Kind: FrameData, Payload: payload})
}

func (s *BidiStream[Req, Resp]) Recv() (*Resp, error) {
	pkt, ok := <-s.recvCh
	if !ok {
		return nil, errors.New("stream closed")
	}
	switch pkt.Kind {
	case FrameData:
		var out Resp
		if err := s.peer.userUnmarshal(pkt.Payload, &out); err != nil {
			return nil, err
		}
		return &out, nil
	case FrameTrailers:
		st, err := s.peer.protoUnmarshalStatus(pkt.Payload)
		if err != nil {
			return nil, err
		}
		if st.IsOK() {
			return nil, nil
		}
		return nil, NewRpcError(st.Code, st.Message)
	default:
		return nil, errors.New("unknown frame kind")
	}
}

func (s *BidiStream[Req, Resp]) CloseSend() error {
	s.mu.Lock()
	if s.closed {
		s.mu.Unlock()
		return nil
	}
	s.closed = true
	s.mu.Unlock()
	st := StatusOKValue()
	payload, err := s.peer.protoMarshal(&st)
	if err != nil {
		return err
	}
	return s.peer.sendPacket(Packet{StreamID: s.streamID, Kind: FrameTrailers, Payload: payload})
}
