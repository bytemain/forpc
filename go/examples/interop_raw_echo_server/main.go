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

	forpc.RegisterRaw(p, *method, func(payload []byte, _ map[string]string, _ *forpc.RpcPeer) ([]byte, *forpc.RpcError) {
		return payload, nil
	})

	if err := p.Serve(); err != nil {
		log.Fatalf("serve: %v", err)
	}
}
