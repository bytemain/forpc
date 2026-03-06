package main

import (
	"flag"
	"fmt"
	"log"

	"github.com/bytemain/forpc/go/forpc"
	"github.com/bytemain/forpc/go/forpc/pb"
	"github.com/vmihailenco/msgpack/v5"
)

type EchoMsg struct {
	Message string `msgpack:"message"`
}

func main() {
	url := flag.String("listen", "tcp://127.0.0.1:24010", "listen url")
	flag.Parse()

	l, err := forpc.Bind(*url)
	if err != nil {
		log.Fatalf("bind: %v", err)
	}
	defer l.Close()

	fmt.Printf("go msgpack echo server listening on %s\n", *url)
	p, err := l.Accept()
	if err != nil {
		log.Fatalf("accept: %v", err)
	}
	defer p.Close()

	p.Register("MsgPack/Echo", func(req forpc.Request, _ *forpc.RpcPeer) forpc.Response {
		var payload []byte
		for pkt := range req.Stream {
			if pkt.Kind == forpc.FrameData {
				payload = pkt.Payload
			} else if pkt.Kind == forpc.FrameTrailers {
				break
			}
		}
		if len(payload) == 0 {
			return forpc.ResponseError(pb.StatusCode_INVALID_ARGUMENT, "Missing payload")
		}

		var msg EchoMsg
		if err := msgpack.Unmarshal(payload, &msg); err != nil {
			return forpc.ResponseError(pb.StatusCode_INVALID_ARGUMENT, fmt.Sprintf("msgpack decode: %v", err))
		}
		fmt.Printf("received: %s\n", msg.Message)

		resp, err := msgpack.Marshal(&msg)
		if err != nil {
			return forpc.ResponseError(pb.StatusCode_INTERNAL, fmt.Sprintf("msgpack encode: %v", err))
		}
		return forpc.ResponseOK(resp)
	})

	if err := p.Serve(); err != nil {
		log.Fatalf("serve: %v", err)
	}
}
