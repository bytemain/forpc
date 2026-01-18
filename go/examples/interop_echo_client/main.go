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
	url := flag.String("connect", "tcp://127.0.0.1:24000", "server url")
	msg := flag.String("msg", "Hello", "message")
	flag.Parse()

	p, err := minirpc.ConnectWithRetry(*url, 50)
	if err != nil {
		log.Fatalf("connect: %v", err)
	}
	defer p.Close()

	if err := minirpc.RegisterTypeByNamespace[EchoRequest](p, "mini_rpc.it", "EchoRequest"); err != nil {
		log.Fatalf("register: %v", err)
	}
	if err := minirpc.RegisterTypeByNamespace[EchoResponse](p, "mini_rpc.it", "EchoResponse"); err != nil {
		log.Fatalf("register: %v", err)
	}

	go func() { _ = p.Serve() }()

	resp, err := minirpc.CallUnary[EchoRequest, EchoResponse](p, "Test/Echo", &EchoRequest{Data: *msg})
	if err != nil {
		log.Fatalf("call: %v", err)
	}
	fmt.Printf("reply: %s\n", resp.Result)
}

