package main

import (
	"flag"
	"fmt"
	"log"

	"github.com/bytemain/forpc/go/forpc"
)

func main() {
	url := flag.String("listen", "tcp://127.0.0.1:24002", "listen url")
	method := flag.String("method", "Raw/Echo", "method")
	flag.Parse()

	l, err := forpc.Bind(*url)
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

	p.Register(*method, func(req forpc.Request, _ *forpc.RpcPeer) forpc.Response {
		var payload []byte
		for pkt := range req.Stream {
			if pkt.Kind == forpc.FrameData {
				payload = pkt.Payload
			} else if pkt.Kind == forpc.FrameTrailers {
				break
			}
		}
		if len(payload) == 0 {
			return forpc.ResponseError(forpc.StatusInvalidArgument, "Missing payload")
		}
		return forpc.ResponseOK(payload)
	})

	if err := p.Serve(); err != nil {
		log.Fatalf("serve: %v", err)
	}
}
