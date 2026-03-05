package forpc

import (
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
