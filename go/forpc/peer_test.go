package forpc

import (
	"encoding/binary"
	"errors"
	"testing"
	"time"

	"google.golang.org/protobuf/proto"

	"github.com/bytemain/forpc/go/forpc/pb"
)

func TestUnaryCallInproc(t *testing.T) {
	url := "inproc://forpc_go_unary"

	l, err := Bind(url)
	if err != nil {
		t.Fatalf("bind: %v", err)
	}
	defer l.Close()

	go func() {
		p, err := l.Accept()
		if err != nil {
			return
		}

		RegisterUnary[pb.TestRequest, pb.TestResponse](p, "Test/Echo", func(req *pb.TestRequest, _ map[string]string, _ *RpcPeer) (*pb.TestResponse, *RpcError) {
			return &pb.TestResponse{Result: req.Data}, nil
		})

		_ = p.Serve()
	}()

	c, err := ConnectWithRetry(url, 10)
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	defer c.Close()

	go func() { _ = c.Serve() }()

	type result struct {
		resp *pb.TestResponse
		err  error
	}
	ch := make(chan result, 1)
	go func() {
		resp, err := CallUnary[pb.TestRequest, pb.TestResponse](c, "Test/Echo", &pb.TestRequest{Data: "Hello"})
		ch <- result{resp: resp, err: err}
	}()
	select {
	case r := <-ch:
		if r.err != nil {
			t.Fatalf("call: %v", r.err)
		}
		if r.resp.Result != "Hello" {
			t.Fatalf("unexpected resp: %#v", r.resp)
		}
	case <-time.After(2 * time.Second):
		t.Fatalf("call timeout")
	}
}

func TestUnaryCallDeadlineExceeded(t *testing.T) {
	url := "inproc://forpc_go_timeout"

	l, err := Bind(url)
	if err != nil {
		t.Fatalf("bind: %v", err)
	}
	defer l.Close()

	go func() {
		p, err := l.Accept()
		if err != nil {
			return
		}

		RegisterUnary[pb.TestRequest, pb.TestResponse](p, "Test/Slow", func(req *pb.TestRequest, _ map[string]string, _ *RpcPeer) (*pb.TestResponse, *RpcError) {
			time.Sleep(5 * time.Second)
			return &pb.TestResponse{Result: req.Data}, nil
		})

		_ = p.Serve()
	}()

	c, err := ConnectWithRetry(url, 10)
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	defer c.Close()

	go func() { _ = c.Serve() }()

	meta := map[string]string{":timeout": "100"}
	_, callErr := CallUnaryWithMetadata[pb.TestRequest, pb.TestResponse](c, "Test/Slow", &pb.TestRequest{Data: "Hello"}, meta)
	if callErr == nil {
		t.Fatalf("expected deadline exceeded error, got nil")
	}
	var rpcErr *RpcError
	if !errors.As(callErr, &rpcErr) {
		t.Fatalf("expected RpcError, got %T: %v", callErr, callErr)
	}
	if rpcErr.Code != pb.StatusCode_DEADLINE_EXCEEDED {
		t.Fatalf("expected DEADLINE_EXCEEDED, got %v", rpcErr.Code)
	}
}

func TestUnaryCallCompletesBeforeTimeout(t *testing.T) {
	url := "inproc://forpc_go_timeout_ok"

	l, err := Bind(url)
	if err != nil {
		t.Fatalf("bind: %v", err)
	}
	defer l.Close()

	go func() {
		p, err := l.Accept()
		if err != nil {
			return
		}

		RegisterUnary[pb.TestRequest, pb.TestResponse](p, "Test/Echo", func(req *pb.TestRequest, _ map[string]string, _ *RpcPeer) (*pb.TestResponse, *RpcError) {
			return &pb.TestResponse{Result: req.Data}, nil
		})

		_ = p.Serve()
	}()

	c, err := ConnectWithRetry(url, 10)
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	defer c.Close()

	go func() { _ = c.Serve() }()

	meta := map[string]string{":timeout": "5000"}
	resp, callErr := CallUnaryWithMetadata[pb.TestRequest, pb.TestResponse](c, "Test/Echo", &pb.TestRequest{Data: "Fast"}, meta)
	if callErr != nil {
		t.Fatalf("unexpected error: %v", callErr)
	}
	if resp.Result != "Fast" {
		t.Fatalf("unexpected resp: %#v", resp)
	}
}

func TestBidiStreamInproc(t *testing.T) {
	url := "inproc://forpc_go_bidi"

	l, err := Bind(url)
	if err != nil {
		t.Fatalf("bind: %v", err)
	}
	defer l.Close()

	go func() {
		p, err := l.Accept()
		if err != nil {
			return
		}
		p.Register("Chat/Connect", func(r Request, peer *RpcPeer) Response {
			for pkt := range r.Stream {
				if pkt.Kind != FrameData {
					continue
				}
				var msg pb.ChatMessage
				if err := proto.Unmarshal(pkt.Payload, &msg); err != nil {
					return ResponseError(pb.StatusCode_INVALID_ARGUMENT, err.Error())
				}
				out, err := proto.Marshal(&pb.ChatMessage{Text: "Echo: " + msg.Text})
				if err != nil {
					return ResponseError(pb.StatusCode_INTERNAL, err.Error())
				}
				_ = peer.sendPacket(Packet{StreamID: r.StreamID, Kind: FrameData, Payload: out})
			}
			return ResponseOK(nil)
		})
		_ = p.Serve()
	}()

	c, err := ConnectWithRetry(url, 10)
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	defer c.Close()

	go func() { _ = c.Serve() }()

	stream, err := Stream[pb.ChatMessage, pb.ChatMessage](c, "Chat/Connect")
	if err != nil {
		t.Fatalf("stream: %v", err)
	}

	for i := 0; i < 3; i++ {
		if err := stream.Send(&pb.ChatMessage{Text: "Msg"}); err != nil {
			t.Fatalf("send: %v", err)
		}
	}
	_ = stream.CloseSend()

	received := 0
	timeout := time.After(2 * time.Second)
	for {
		select {
		case <-timeout:
			t.Fatalf("recv timeout")
		case pkt, ok := <-stream.recvCh:
			if !ok {
				goto done
			}
			switch pkt.Kind {
			case FrameData:
				var msg pb.ChatMessage
				if err := proto.Unmarshal(pkt.Payload, &msg); err != nil {
					t.Fatalf("unmarshal: %v", err)
				}
				if msg.Text == "" {
					t.Fatalf("empty msg")
				}
				received++
			case FrameTrailers:
				st, err := stream.peer.protoUnmarshalStatus(pkt.Payload)
				if err != nil {
					t.Fatalf("status: %v", err)
				}
				if !st.IsOK() {
					t.Fatalf("status not ok: %d %s", st.Code, st.Message)
				}
				goto done
			}
		}
	}
done:
	if received != 3 {
		t.Fatalf("expected 3, got %d", received)
	}
}

func TestRstStreamCancelsServerHandler(t *testing.T) {
	url := "inproc://forpc_go_rst_cancel"

	l, err := Bind(url)
	if err != nil {
		t.Fatalf("bind: %v", err)
	}
	defer l.Close()

	cancelObserved := make(chan struct{}, 1)

	go func() {
		p, err := l.Accept()
		if err != nil {
			return
		}

		// Register a slow handler that checks for cancellation via context
		p.Register("Test/SlowCancel", func(r Request, _ *RpcPeer) Response {
			select {
			case <-r.Ctx.Done():
				cancelObserved <- struct{}{}
				return ResponseError(pb.StatusCode_CANCELLED, "Cancelled")
			case <-time.After(10 * time.Second):
				return ResponseOK([]byte("done"))
			}
		})

		_ = p.Serve()
	}()

	c, err := ConnectWithRetry(url, 10)
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	defer c.Close()

	go func() { _ = c.Serve() }()

	// Call with short timeout → triggers RST_STREAM on timeout
	meta := map[string]string{":timeout": "100"}
	_, callErr := c.CallRawWithMetadata("Test/SlowCancel", []byte("test"), meta)
	if callErr == nil {
		t.Fatalf("expected deadline exceeded error, got nil")
	}
	var rpcErr *RpcError
	if !errors.As(callErr, &rpcErr) {
		t.Fatalf("expected RpcError, got %T: %v", callErr, callErr)
	}
	if rpcErr.Code != pb.StatusCode_DEADLINE_EXCEEDED {
		t.Fatalf("expected DEADLINE_EXCEEDED, got %v", rpcErr.Code)
	}

	// Wait for the server handler to observe cancellation
	select {
	case <-cancelObserved:
		// success
	case <-time.After(2 * time.Second):
		t.Fatalf("server handler did not observe cancellation")
	}
}

func TestRstStreamPacketRoundtrip(t *testing.T) {
	streamID := uint32(42)
	errorCode := uint32(pb.StatusCode_CANCELLED)
	payload := make([]byte, 4)
	binary.BigEndian.PutUint32(payload, errorCode)
	pkt := Packet{StreamID: streamID, Kind: FrameRstStream, Payload: payload}
	encoded, err := pkt.Encode()
	if err != nil {
		t.Fatalf("encode: %v", err)
	}
	decoded, err := DecodePacket(encoded)
	if err != nil {
		t.Fatalf("decode: %v", err)
	}
	if decoded.StreamID != streamID {
		t.Fatalf("expected stream_id %d, got %d", streamID, decoded.StreamID)
	}
	if decoded.Kind != FrameRstStream {
		t.Fatalf("expected kind %d, got %d", FrameRstStream, decoded.Kind)
	}
	if len(decoded.Payload) != 4 {
		t.Fatalf("expected 4 bytes payload, got %d", len(decoded.Payload))
	}
	decodedCode := binary.BigEndian.Uint32(decoded.Payload)
	if decodedCode != errorCode {
		t.Fatalf("expected error code %d, got %d", errorCode, decodedCode)
	}
}

func TestReadPayload(t *testing.T) {
	// ReadPayload with Payload already set returns it directly.
	r := &Request{Payload: []byte("direct")}
	if got := r.ReadPayload(); string(got) != "direct" {
		t.Fatalf("expected 'direct', got %q", got)
	}

	// ReadPayload with no Stream and nil Payload returns nil.
	r2 := &Request{}
	if got := r2.ReadPayload(); got != nil {
		t.Fatalf("expected nil, got %v", got)
	}

	// ReadPayload extracts DATA frame from Stream channel.
	ch := make(chan Packet, 2)
	ch <- Packet{Kind: FrameData, Payload: []byte("from-stream")}
	ch <- Packet{Kind: FrameTrailers, Payload: []byte{}}
	close(ch)
	r3 := &Request{Stream: ch}
	if got := r3.ReadPayload(); string(got) != "from-stream" {
		t.Fatalf("expected 'from-stream', got %q", got)
	}
}

func TestRegisterRawInproc(t *testing.T) {
	url := "inproc://forpc_go_raw"

	l, err := Bind(url)
	if err != nil {
		t.Fatalf("bind: %v", err)
	}
	defer l.Close()

	go func() {
		p, err := l.Accept()
		if err != nil {
			return
		}

		RegisterRaw(p, "Raw/Echo", func(payload []byte, _ map[string]string, _ *RpcPeer) ([]byte, *RpcError) {
			return payload, nil
		})

		_ = p.Serve()
	}()

	c, err := ConnectWithRetry(url, 10)
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	defer c.Close()

	go func() { _ = c.Serve() }()

	type result struct {
		resp []byte
		err  error
	}
	ch := make(chan result, 1)
	go func() {
		resp, err := c.CallRaw("Raw/Echo", []byte("Hello Raw"))
		ch <- result{resp: resp, err: err}
	}()
	select {
	case r := <-ch:
		if r.err != nil {
			t.Fatalf("call: %v", r.err)
		}
		if string(r.resp) != "Hello Raw" {
			t.Fatalf("unexpected resp: %q", string(r.resp))
		}
	case <-time.After(2 * time.Second):
		t.Fatalf("call timeout")
	}
}

func TestBidiStreamCancel(t *testing.T) {
	url := "inproc://forpc_go_bidi_cancel"

	l, err := Bind(url)
	if err != nil {
		t.Fatalf("bind: %v", err)
	}
	defer l.Close()

	cancelObserved := make(chan struct{}, 1)

	go func() {
		p, err := l.Accept()
		if err != nil {
			return
		}
		// Register a handler that waits for context cancellation
		p.Register("Chat/Stream", func(r Request, _ *RpcPeer) Response {
			select {
			case <-r.Ctx.Done():
				cancelObserved <- struct{}{}
				return ResponseError(pb.StatusCode_CANCELLED, "Cancelled")
			case <-time.After(10 * time.Second):
				return ResponseOK([]byte("done"))
			}
		})
		_ = p.Serve()
	}()

	c, err := ConnectWithRetry(url, 10)
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	defer c.Close()

	go func() { _ = c.Serve() }()

	stream, err := Stream[pb.ChatMessage, pb.ChatMessage](c, "Chat/Stream")
	if err != nil {
		t.Fatalf("stream: %v", err)
	}

	// Send a message then cancel
	if err := stream.Send(&pb.ChatMessage{Text: "hello"}); err != nil {
		t.Fatalf("send: %v", err)
	}

	stream.Cancel()

	// Wait for the server handler to observe cancellation via context
	select {
	case <-cancelObserved:
		// success
	case <-time.After(2 * time.Second):
		t.Fatalf("server handler did not observe stream cancellation")
	}
}
