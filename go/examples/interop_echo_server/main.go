package main

import (
	"flag"
	"fmt"
	"log"

	"github.com/bytemain/forpc/go/forpc"
	"github.com/bytemain/forpc/go/forpc/pb"
)

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

	forpc.RegisterUnary[pb.EchoRequest, pb.EchoResponse](p, "Test/Echo", func(req *pb.EchoRequest, _ map[string]string, _ *forpc.RpcPeer) (*pb.EchoResponse, *forpc.RpcError) {
		return &pb.EchoResponse{Result: req.Data}, nil
	})

	if err := p.Serve(); err != nil {
		log.Fatalf("serve: %v", err)
	}
}
