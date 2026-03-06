package main

import (
	"flag"
	"fmt"
	"log"

	"github.com/bytemain/forpc/go/forpc"
	"github.com/vmihailenco/msgpack/v5"
)

type EchoMsg struct {
	Message string `msgpack:"message"`
}

func main() {
	url := flag.String("connect", "tcp://127.0.0.1:24010", "server url")
	msg := flag.String("msg", "Hello", "message")
	flag.Parse()

	p, err := forpc.ConnectWithRetry(*url, 50)
	if err != nil {
		log.Fatalf("connect: %v", err)
	}
	defer p.Close()

	go func() { _ = p.Serve() }()

	payload, err := msgpack.Marshal(&EchoMsg{Message: *msg})
	if err != nil {
		log.Fatalf("msgpack encode: %v", err)
	}

	b, err := p.CallRaw("MsgPack/Echo", payload)
	if err != nil {
		log.Fatalf("call: %v", err)
	}

	var resp EchoMsg
	if err := msgpack.Unmarshal(b, &resp); err != nil {
		log.Fatalf("msgpack decode: %v", err)
	}
	fmt.Printf("reply: %s\n", resp.Message)
}
