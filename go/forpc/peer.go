package forpc

import (
	"errors"
	"sync"
	"sync/atomic"
	"time"

	"github.com/apache/fory/go/fory"

	"github.com/bytemain/forpc/go/transport"
	mangostransport "github.com/bytemain/forpc/go/transport/mangos"
)

type Request struct {
	Method   string
	Metadata map[string]string
	Payload  []byte
	Stream   <-chan Packet
	StreamID uint32
}

type Response struct {
	Metadata map[string]string
	Payload  []byte
	Status   Status
}

func ResponseOK(payload []byte) Response {
	return Response{Metadata: map[string]string{}, Payload: payload, Status: StatusOKValue()}
}

func ResponseError(code uint32, message string) Response {
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

	nextStreamID atomic.Uint32
	isInitiator  bool

	protoMu   sync.Mutex
	protoFory *fory.Fory
	userMu    sync.Mutex
	userFory  *fory.Fory

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
	protoF := fory.New(fory.WithCompatible(true), fory.WithXlang(true))
	_ = protoF.RegisterStruct(Call{}, 2)
	_ = protoF.RegisterStruct(Status{}, 3)

	userF := fory.New(fory.WithCompatible(true), fory.WithXlang(true))

	p := &RpcPeer{
		transport:   t,
		handlers:    make(map[string]handlerFunc),
		pending:     make(map[uint32]*pendingCall),
		inbound:     make(map[uint32]*inboundState),
		isInitiator: isInitiator,
		protoFory:   protoF,
		userFory:    userF,
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

func (p *RpcPeer) RegisterStruct(v any, typeID uint32) error {
	p.userMu.Lock()
	defer p.userMu.Unlock()
	return p.userFory.RegisterStruct(v, typeID)
}

func (p *RpcPeer) RegisterNamedStruct(type_ any, typeName string) error {
	p.userMu.Lock()
	defer p.userMu.Unlock()
	return p.userFory.RegisterNamedStruct(type_, typeName)
}

func (p *RpcPeer) RegisterTypeByNamespace(type_ any, namespace string, name string) error {
	if namespace == "" {
		return p.RegisterNamedStruct(type_, name)
	}
	return p.RegisterNamedStruct(type_, namespace+"."+name)
}

func (p *RpcPeer) Register(method string, h handlerFunc) {
	p.handlersMu.Lock()
	p.handlers[method] = h
	p.handlersMu.Unlock()
}

func (p *RpcPeer) Call(method string, req any, resp any) error {
	return p.CallWithMetadata(method, req, map[string]string{}, resp)
}

func (p *RpcPeer) CallWithMetadata(method string, req any, meta map[string]string, resp any) error {
	dataPayload, err := p.userMarshal(req)
	if err != nil {
		return err
	}
	resBytes, err := p.unaryRawWithMetadata(method, meta, dataPayload)
	if err != nil {
		return err
	}
	return p.userUnmarshal(resBytes, resp)
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

	call := Call{Method: method, Metadata: meta}
	hdrPayload, err := p.protoMarshal(&call)
	if err != nil {
		return nil, err
	}
	if err := p.sendPacket(Packet{StreamID: streamID, Kind: FrameHeaders, Payload: hdrPayload}); err != nil {
		return nil, err
	}
	if err := p.sendPacket(Packet{StreamID: streamID, Kind: FrameData, Payload: payload}); err != nil {
		return nil, err
	}

	eos := StatusOKValue()
	trPayload, err := p.protoMarshal(&eos)
	if err != nil {
		return nil, err
	}
	if err := p.sendPacket(Packet{StreamID: streamID, Kind: FrameTrailers, Payload: trPayload}); err != nil {
		return nil, err
	}
	cleanup = false

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

	call := Call{Method: method, Metadata: meta}
	hdrPayload, err := p.protoMarshal(&call)
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
			_ = p.sendResponse(pkt.StreamID, ResponseError(StatusUnimplemented, "method not found"))
			return nil
		}
		rx := make(chan Packet, 32)
		p.inboundMu.Lock()
		p.inbound[pkt.StreamID] = &inboundState{tx: rx}
		p.inboundMu.Unlock()

		req := Request{
			Method:   call.Method,
			Metadata: call.Metadata,
			Payload:  nil,
			Stream:   rx,
			StreamID: pkt.StreamID,
		}

		go func() {
			resp := h(req, p)
			_ = p.sendResponse(pkt.StreamID, resp)
		}()

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
	tr, err := p.protoMarshal(&resp.Status)
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

func (p *RpcPeer) allocStreamID() uint32 {
	return p.nextStreamID.Add(2) - 2
}

func (p *RpcPeer) removePending(streamID uint32) {
	p.pendingMu.Lock()
	delete(p.pending, streamID)
	p.pendingMu.Unlock()
}

func (p *RpcPeer) protoMarshal(v any) ([]byte, error) {
	p.protoMu.Lock()
	defer p.protoMu.Unlock()
	return p.protoFory.Marshal(v)
}

func (p *RpcPeer) userMarshal(v any) ([]byte, error) {
	p.userMu.Lock()
	defer p.userMu.Unlock()
	return p.userFory.Marshal(v)
}

func (p *RpcPeer) protoUnmarshalCall(data []byte) (*Call, error) {
	var c Call
	p.protoMu.Lock()
	err := p.protoFory.Unmarshal(data, &c)
	p.protoMu.Unlock()
	if err != nil {
		return nil, err
	}
	if c.Metadata == nil {
		c.Metadata = map[string]string{}
	}
	return &c, nil
}

func (p *RpcPeer) protoUnmarshalStatus(data []byte) (*Status, error) {
	var st Status
	p.protoMu.Lock()
	err := p.protoFory.Unmarshal(data, &st)
	p.protoMu.Unlock()
	if err != nil {
		return nil, err
	}
	return &st, nil
}

func (p *RpcPeer) userUnmarshal(data []byte, out any) error {
	p.userMu.Lock()
	defer p.userMu.Unlock()
	return p.userFory.Unmarshal(data, out)
}

func (p *RpcPeer) RequireStructRegistered(typeID uint32, v any) error {
	if typeID == 0 {
		return errors.New("type id must be non-zero")
	}
	return p.RegisterStruct(v, typeID)
}
