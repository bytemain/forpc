package main

import (
	"flag"
	"fmt"
	"log"

	"github.com/bytemain/forpc/go/forpc"
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

	l, err := forpc.Bind(*url)
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

	if err := forpc.RegisterTypeByNamespace[EchoRequest](p, "forpc.it", "EchoRequest"); err != nil {
		log.Fatalf("register: %v", err)
	}
	if err := forpc.RegisterTypeByNamespace[EchoResponse](p, "forpc.it", "EchoResponse"); err != nil {
		log.Fatalf("register: %v", err)
	}

	forpc.RegisterUnary[EchoRequest, EchoResponse](p, "Test/Echo", func(req *EchoRequest, _ map[string]string, _ *forpc.RpcPeer) (*EchoResponse, *forpc.RpcError) {
		return &EchoResponse{Result: req.Data}, nil
	})

	if err := p.Serve(); err != nil {
		log.Fatalf("serve: %v", err)
	}
}
