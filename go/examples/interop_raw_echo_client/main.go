package main

import (
	"flag"
	"fmt"
	"log"

	"github.com/bytemain/forpc/go/forpc"
)

func main() {
	url := flag.String("connect", "tcp://127.0.0.1:24002", "server url")
	method := flag.String("method", "Raw/Echo", "method")
	msg := flag.String("msg", "Hello", "message")
	flag.Parse()

	p, err := forpc.ConnectWithRetry(*url, 50)
	if err != nil {
		log.Fatalf("connect: %v", err)
	}
	defer p.Close()

	go func() { _ = p.Serve() }()

	b, err := p.CallRaw(*method, []byte(*msg))
	if err != nil {
		log.Fatalf("call: %v", err)
	}
	fmt.Printf("reply: %s\n", string(b))
}
