package forpc

import (
	"context"
	"encoding/binary"
	"strconv"
	"sync"
	"sync/atomic"
	"time"

	"google.golang.org/protobuf/proto"

	"github.com/bytemain/forpc/go/forpc/pb"
	"github.com/bytemain/forpc/go/transport"
	mangostransport "github.com/bytemain/forpc/go/transport/mangos"
)

type Request struct {
	Method   string
	Metadata map[string]string
	Payload  []byte
	Stream   <-chan Packet
	StreamID uint32
	Ctx      context.Context
}

// ReadPayload returns the request payload bytes. If Payload is already
// populated it is returned directly. Otherwise the method drains the
// Stream channel, returning the last DATA frame payload and discarding
// protocol frames so callers do not need to understand the frame structure.
// For unary calls the protocol sends a single DATA frame, so this returns
// that frame's payload. If multiple DATA frames are present, only the
// last one is kept.
func (r *Request) ReadPayload() []byte {
	if r.Payload != nil {
		return r.Payload
	}
	if r.Stream == nil {
		return nil
	}
	var payload []byte
	for pkt := range r.Stream {
		if pkt.Kind == FrameData {
			payload = pkt.Payload
		}
	}
	return payload
}

type Response struct {
	Metadata map[string]string
	Payload  []byte
	Status   Status
}

func ResponseOK(payload []byte) Response {
	return Response{Metadata: map[string]string{}, Payload: payload, Status: StatusOKValue()}
}

func ResponseError(code pb.StatusCode, message string) Response {
	return Response{Metadata: map[string]string{}, Payload: nil, Status: Status{Code: code, Message: message}}
}

type handlerFunc func(Request, *RpcPeer) Response

type pendingCall struct {
	unaryCh    chan resultBytes
	streamCh   chan Packet
	unaryBuf   []byte
	unaryBufMu sync.Mutex
}

type resultBytes struct {
	b   []byte
	err error
}

type inboundState struct {
	tx chan Packet
}

type RpcPeer struct {
	transport transport.Transport

	handlersMu sync.RWMutex
	handlers   map[string]handlerFunc

	pendingMu sync.Mutex
	pending   map[uint32]*pendingCall

	inboundMu sync.Mutex
	inbound   map[uint32]*inboundState

	cancelsMu sync.Mutex
	cancels   map[uint32]context.CancelFunc

	nextStreamID atomic.Uint32
	isInitiator  bool

	running atomic.Bool
}

func Connect(url string) (*RpcPeer, error) {
	t, err := mangostransport.NewClientTransport(url)
	if err != nil {
		return nil, err
	}
	return NewPeer(t, true), nil
}

func ConnectWithRetry(url string, maxRetries uint32) (*RpcPeer, error) {
	t, err := mangostransport.NewClientTransportWithRetry(url, maxRetries, 100*time.Millisecond)
	if err != nil {
		return nil, err
	}
	return NewPeer(t, true), nil
}

func NewPeer(t transport.Transport, isInitiator bool) *RpcPeer {
	p := &RpcPeer{
		transport:   t,
		handlers:    make(map[string]handlerFunc),
		pending:     make(map[uint32]*pendingCall),
		inbound:     make(map[uint32]*inboundState),
		cancels:     make(map[uint32]context.CancelFunc),
		isInitiator: isInitiator,
	}
	if isInitiator {
		p.nextStreamID.Store(1)
	} else {
		p.nextStreamID.Store(2)
	}
	p.running.Store(true)
	return p
}

func (p *RpcPeer) Close() error {
	p.running.Store(false)
	return p.transport.Close()
}

func (p *RpcPeer) Register(method string, h handlerFunc) {
	p.handlersMu.Lock()
	p.handlers[method] = h
	p.handlersMu.Unlock()
}

func (p *RpcPeer) Call(method string, req proto.Message, resp proto.Message) error {
	return p.CallWithMetadata(method, req, map[string]string{}, resp)
}

func (p *RpcPeer) CallWithMetadata(method string, req proto.Message, meta map[string]string, resp proto.Message) error {
	dataPayload, err := proto.Marshal(req)
	if err != nil {
		return err
	}
	resBytes, err := p.unaryRawWithMetadata(method, meta, dataPayload)
	if err != nil {
		return err
	}
	return proto.Unmarshal(resBytes, resp)
}

func (p *RpcPeer) CallRaw(method string, payload []byte) ([]byte, error) {
	return p.CallRawWithMetadata(method, payload, map[string]string{})
}

func (p *RpcPeer) CallRawWithMetadata(method string, payload []byte, meta map[string]string) ([]byte, error) {
	return p.unaryRawWithMetadata(method, meta, payload)
}

func (p *RpcPeer) unaryRawWithMetadata(method string, meta map[string]string, payload []byte) ([]byte, error) {
	streamID := p.allocStreamID()

	pc := &pendingCall{unaryCh: make(chan resultBytes, 1)}
	cleanup := true
	defer func() {
		if cleanup {
			p.removePending(streamID)
		}
	}()
	p.pendingMu.Lock()
	p.pending[streamID] = pc
	p.pendingMu.Unlock()

	call := &pb.Call{Method: method, Metadata: meta}
	hdrPayload, err := proto.Marshal(call)
	if err != nil {
		return nil, err
	}
	if err := p.sendPacket(Packet{StreamID: streamID, Kind: FrameHeaders, Payload: hdrPayload}); err != nil {
		return nil, err
	}
	if err := p.sendPacket(Packet{StreamID: streamID, Kind: FrameData, Payload: payload}); err != nil {
		return nil, err
	}

	eos := &pb.Status{Code: pb.StatusCode_OK, Message: "OK"}
	trPayload, err := proto.Marshal(eos)
	if err != nil {
		return nil, err
	}
	if err := p.sendPacket(Packet{StreamID: streamID, Kind: FrameTrailers, Payload: trPayload}); err != nil {
		return nil, err
	}
	cleanup = false

	var timeoutCh <-chan time.Time
	if ts, ok := meta[":timeout"]; ok {
		if ms, err := strconv.ParseInt(ts, 10, 64); err == nil && ms > 0 {
			timeoutCh = time.After(time.Duration(ms) * time.Millisecond)
		}
	}

	if timeoutCh != nil {
		select {
		case res := <-pc.unaryCh:
			if res.err != nil {
				return nil, res.err
			}
			return res.b, nil
		case <-timeoutCh:
			p.removePending(streamID)
			p.sendRstStream(streamID, uint32(pb.StatusCode_CANCELLED))
			return nil, NewRpcError(pb.StatusCode_DEADLINE_EXCEEDED, "Deadline exceeded")
		}
	}

	res := <-pc.unaryCh
	if res.err != nil {
		return nil, res.err
	}
	return res.b, nil
}

func (p *RpcPeer) streamInternal(method string, meta map[string]string) (uint32, <-chan Packet, error) {
	streamID := p.allocStreamID()

	ch := make(chan Packet, 32)
	pc := &pendingCall{streamCh: ch}
	cleanup := true
	defer func() {
		if cleanup {
			p.removePending(streamID)
		}
	}()
	p.pendingMu.Lock()
	p.pending[streamID] = pc
	p.pendingMu.Unlock()

	call := &pb.Call{Method: method, Metadata: meta}
	hdrPayload, err := proto.Marshal(call)
	if err != nil {
		return 0, nil, err
	}
	if err := p.sendPacket(Packet{StreamID: streamID, Kind: FrameHeaders, Payload: hdrPayload}); err != nil {
		return 0, nil, err
	}
	cleanup = false

	return streamID, ch, nil
}

func (p *RpcPeer) Serve() error {
	for p.running.Load() {
		b, err := p.transport.Recv()
		if err != nil {
			if p.running.Load() {
				return err
			}
			break
		}
		pkt, err := DecodePacket(b)
		if err != nil {
			continue
		}
		if pkt.StreamID == 0 {
			continue
		}
		isInbound := false
		if p.isInitiator {
			isInbound = pkt.StreamID%2 == 0
		} else {
			isInbound = pkt.StreamID%2 == 1
		}
		if isInbound {
			_ = p.handleInbound(pkt)
		} else {
			_ = p.handleOutbound(pkt)
		}
	}
	return nil
}

func (p *RpcPeer) handleInbound(pkt Packet) error {
	switch pkt.Kind {
	case FrameHeaders:
		call, err := p.protoUnmarshalCall(pkt.Payload)
		if err != nil {
			return err
		}
		p.handlersMu.RLock()
		h := p.handlers[call.Method]
		p.handlersMu.RUnlock()
		if h == nil {
			_ = p.sendResponse(pkt.StreamID, ResponseError(pb.StatusCode_UNIMPLEMENTED, "method not found"))
			return nil
		}
		ctx, cancel := context.WithCancel(context.Background())
		rx := make(chan Packet, 32)
		p.inboundMu.Lock()
		p.inbound[pkt.StreamID] = &inboundState{tx: rx}
		p.inboundMu.Unlock()
		p.cancelsMu.Lock()
		p.cancels[pkt.StreamID] = cancel
		p.cancelsMu.Unlock()

		req := Request{
			Method:   call.Method,
			Metadata: call.Metadata,
			Payload:  nil,
			Stream:   rx,
			StreamID: pkt.StreamID,
			Ctx:      ctx,
		}

		go func() {
			resp := h(req, p)
			// Clean up cancel function after handler completes
			p.cancelsMu.Lock()
			delete(p.cancels, pkt.StreamID)
			p.cancelsMu.Unlock()
			_ = p.sendResponse(pkt.StreamID, resp)
		}()

	case FrameRstStream:
		p.cancelsMu.Lock()
		cancelFn := p.cancels[pkt.StreamID]
		delete(p.cancels, pkt.StreamID)
		p.cancelsMu.Unlock()
		if cancelFn != nil {
			cancelFn()
		}
		p.inboundMu.Lock()
		st := p.inbound[pkt.StreamID]
		delete(p.inbound, pkt.StreamID)
		p.inboundMu.Unlock()
		if st != nil {
			close(st.tx)
		}

	case FrameData, FrameTrailers:
		p.inboundMu.Lock()
		st := p.inbound[pkt.StreamID]
		if pkt.Kind == FrameTrailers {
			delete(p.inbound, pkt.StreamID)
		}
		p.inboundMu.Unlock()
		if st != nil {
			st.tx <- pkt
			if pkt.Kind == FrameTrailers {
				close(st.tx)
			}
		}
	}
	return nil
}

func (p *RpcPeer) handleOutbound(pkt Packet) error {
	p.pendingMu.Lock()
	pc := p.pending[pkt.StreamID]
	if pc == nil {
		p.pendingMu.Unlock()
		return nil
	}
	switch pkt.Kind {
	case FrameData:
		if pc.streamCh != nil {
			ch := pc.streamCh
			p.pendingMu.Unlock()
			ch <- pkt
			return nil
		}
		pc.unaryBufMu.Lock()
		pc.unaryBuf = make([]byte, len(pkt.Payload))
		copy(pc.unaryBuf, pkt.Payload)
		pc.unaryBufMu.Unlock()
		p.pendingMu.Unlock()
	case FrameTrailers:
		delete(p.pending, pkt.StreamID)
		p.pendingMu.Unlock()
		st, err := p.protoUnmarshalStatus(pkt.Payload)
		if err != nil {
			pc.unaryCh <- resultBytes{b: nil, err: err}
			return nil
		}
		if pc.streamCh != nil {
			pc.streamCh <- pkt
			close(pc.streamCh)
			return nil
		}
		if st.IsOK() {
			pc.unaryBufMu.Lock()
			b := pc.unaryBuf
			pc.unaryBufMu.Unlock()
			pc.unaryCh <- resultBytes{b: b, err: nil}
		} else {
			pc.unaryCh <- resultBytes{b: nil, err: NewRpcError(st.Code, st.Message)}
		}
	case FrameRstStream:
		delete(p.pending, pkt.StreamID)
		p.pendingMu.Unlock()
		if pc.unaryCh != nil {
			pc.unaryCh <- resultBytes{b: nil, err: NewRpcError(pb.StatusCode_CANCELLED, "Stream reset by peer")}
		}
		if pc.streamCh != nil {
			close(pc.streamCh)
		}
	default:
		p.pendingMu.Unlock()
	}
	return nil
}

func (p *RpcPeer) sendResponse(streamID uint32, resp Response) error {
	if len(resp.Payload) > 0 {
		if err := p.sendPacket(Packet{StreamID: streamID, Kind: FrameData, Payload: resp.Payload}); err != nil {
			return err
		}
	}
	st := &pb.Status{Code: resp.Status.Code, Message: resp.Status.Message}
	tr, err := proto.Marshal(st)
	if err != nil {
		return err
	}
	return p.sendPacket(Packet{StreamID: streamID, Kind: FrameTrailers, Payload: tr})
}

func (p *RpcPeer) sendPacket(pkt Packet) error {
	b, err := pkt.Encode()
	if err != nil {
		return err
	}
	return p.transport.Send(b)
}

func (p *RpcPeer) sendRstStream(streamID uint32, errorCode uint32) {
	payload := make([]byte, 4)
	binary.BigEndian.PutUint32(payload, errorCode)
	_ = p.sendPacket(Packet{StreamID: streamID, Kind: FrameRstStream, Payload: payload})
}

func (p *RpcPeer) allocStreamID() uint32 {
	return p.nextStreamID.Add(2) - 2
}

func (p *RpcPeer) removePending(streamID uint32) {
	p.pendingMu.Lock()
	delete(p.pending, streamID)
	p.pendingMu.Unlock()
}

func (p *RpcPeer) protoUnmarshalCall(data []byte) (*Call, error) {
	var c pb.Call
	if err := proto.Unmarshal(data, &c); err != nil {
		return nil, err
	}
	metadata := c.Metadata
	if metadata == nil {
		metadata = map[string]string{}
	}
	return &Call{Method: c.Method, Metadata: metadata}, nil
}

func (p *RpcPeer) protoUnmarshalStatus(data []byte) (*Status, error) {
	var st pb.Status
	if err := proto.Unmarshal(data, &st); err != nil {
		return nil, err
	}
	return &Status{Code: st.Code, Message: st.Message}, nil
}


