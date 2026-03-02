package main

import (
	"flag"
	"fmt"
	"log"

	"github.com/bytemain/forpc/go/forpc"
	"github.com/bytemain/forpc/go/forpc/pb"
)

func main() {
	url := flag.String("connect", "tcp://127.0.0.1:24000", "server url")
	msg := flag.String("msg", "Hello", "message")
	flag.Parse()

	p, err := forpc.ConnectWithRetry(*url, 50)
	if err != nil {
		log.Fatalf("connect: %v", err)
	}
	defer p.Close()

	go func() { _ = p.Serve() }()

	resp, err := forpc.CallUnary[pb.EchoRequest, pb.EchoResponse](p, "Test/Echo", &pb.EchoRequest{Data: *msg})
	if err != nil {
		log.Fatalf("call: %v", err)
	}
	fmt.Printf("reply: %s\n", resp.Result)
}
