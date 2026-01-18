package main

import (
	"flag"
	"fmt"
	"log"

	"github.com/bytemain/mini-rpc/go/minirpc"
)

type EchoRequest struct {
	Data string
}

type EchoResponse struct {
	Result string
}

func main() {
	url := flag.String("listen", "tcp://127.0.0.1:24000", "listen url")
	flag.Parse()

	l, err := minirpc.Bind(*url)
	if err != nil {
		log.Fatalf("bind: %v", err)
	}
	defer l.Close()

	fmt.Printf("go interop echo server listening on %s\n", *url)
	p, err := l.Accept()
	if err != nil {
		log.Fatalf("accept: %v", err)
	}
	defer p.Close()

	if err := minirpc.RegisterTypeByNamespace[EchoRequest](p, "mini_rpc.it", "EchoRequest"); err != nil {
		log.Fatalf("register: %v", err)
	}
	if err := minirpc.RegisterTypeByNamespace[EchoResponse](p, "mini_rpc.it", "EchoResponse"); err != nil {
		log.Fatalf("register: %v", err)
	}

	minirpc.RegisterUnary[EchoRequest, EchoResponse](p, "Test/Echo", func(req *EchoRequest, _ map[string]string, _ *minirpc.RpcPeer) (*EchoResponse, *minirpc.RpcError) {
		return &EchoResponse{Result: req.Data}, nil
	})

	if err := p.Serve(); err != nil {
		log.Fatalf("serve: %v", err)
	}
}

