package main

import (
	"flag"
	"fmt"
	"log"

	"github.com/bytemain/mini-rpc/go/minirpc"
)

func main() {
	url := flag.String("listen", "tcp://127.0.0.1:24002", "listen url")
	method := flag.String("method", "Raw/Echo", "method")
	flag.Parse()

	l, err := minirpc.Bind(*url)
	if err != nil {
		log.Fatalf("bind: %v", err)
	}
	defer l.Close()

	fmt.Printf("go raw echo server listening on %s\n", *url)
	p, err := l.Accept()
	if err != nil {
		log.Fatalf("accept: %v", err)
	}
	defer p.Close()

	p.Register(*method, func(req minirpc.Request, _ *minirpc.RpcPeer) minirpc.Response {
		var payload []byte
		for pkt := range req.Stream {
			if pkt.Kind == minirpc.FrameData {
				payload = pkt.Payload
			} else if pkt.Kind == minirpc.FrameTrailers {
				break
			}
		}
		if len(payload) == 0 {
			return minirpc.ResponseError(minirpc.StatusInvalidArgument, "Missing payload")
		}
		return minirpc.ResponseOK(payload)
	})

	if err := p.Serve(); err != nil {
		log.Fatalf("serve: %v", err)
	}
}

