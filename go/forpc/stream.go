package forpc

import (
	"errors"
	"sync"

	"google.golang.org/protobuf/proto"

	"github.com/bytemain/forpc/go/forpc/pb"
)

type BidiStream[Req any, Resp any] struct {
	streamID uint32
	peer     *RpcPeer
	recvCh   <-chan *Packet
	mu       sync.Mutex
	closed   bool
}

func (s *BidiStream[Req, Resp]) Send(msg *Req) error {
	pm, ok := any(msg).(proto.Message)
	if !ok {
		return errors.New("message does not implement proto.Message")
	}
	payload, err := proto.Marshal(pm)
	if err != nil {
		return err
	}
	return s.peer.sendPacket(DataPacket(s.streamID, payload))
}

func (s *BidiStream[Req, Resp]) Recv() (*Resp, error) {
	pkt, ok := <-s.recvCh
	if !ok {
		return nil, errors.New("stream closed")
	}
	switch pkt.Kind {
	case FrameData:
		var out Resp
		pm, ok := any(&out).(proto.Message)
		if !ok {
			return nil, errors.New("message does not implement proto.Message")
		}
		if err := proto.Unmarshal(pkt.Payload, pm); err != nil {
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
	pkt, err := TrailersPacket(s.streamID, &pb.Status{Code: pb.StatusCode_OK, Message: "OK"})
	if err != nil {
		return err
	}
	return s.peer.sendPacket(pkt)
}

// Cancel sends a RST_STREAM frame to the remote peer, signaling cancellation.
// This removes the pending call and notifies the server to stop processing.
func (s *BidiStream[Req, Resp]) Cancel() {
	s.peer.removePending(s.streamID)
	_ = s.peer.sendPacket(RstStreamPacket(s.streamID, uint32(pb.StatusCode_CANCELLED)))
}
