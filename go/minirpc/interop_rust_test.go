//go:build integration

package minirpc

import (
	"bytes"
	"context"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"testing"
	"time"
)

type interopEchoRequest struct {
	Data string
}

type interopEchoResponse struct {
	Result string
}

func TestInteropRustGoUnary(t *testing.T) {
	url := pickTCPURL(t)

	cmd, stop := startRustInteropServer(t, url)
	defer stop()
	_ = cmd

	c, err := ConnectWithRetry(url, 50)
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	defer c.Close()

	_ = RegisterTypeByNamespace[interopEchoRequest](c, "mini_rpc.it", "EchoRequest")
	_ = RegisterTypeByNamespace[interopEchoResponse](c, "mini_rpc.it", "EchoResponse")

	go func() { _ = c.Serve() }()

	type result struct {
		resp *interopEchoResponse
		err  error
	}
	ch := make(chan result, 1)
	go func() {
		resp, err := CallUnary[interopEchoRequest, interopEchoResponse](c, "Test/Echo", &interopEchoRequest{Data: "Hello"})
		ch <- result{resp: resp, err: err}
	}()
	select {
	case r := <-ch:
		if r.err != nil {
			t.Fatalf("call: %v", r.err)
		}
		if r.resp.Result != "Hello" {
			t.Fatalf("unexpected resp: %#v", r.resp)
		}
	case <-time.After(10 * time.Second):
		stop()
		t.Fatalf("call timeout")
	}
}

func pickTCPURL(t *testing.T) string {
	t.Helper()
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("listen: %v", err)
	}
	addr := ln.Addr().(*net.TCPAddr)
	_ = ln.Close()
	return fmt.Sprintf("tcp://127.0.0.1:%d", addr.Port)
}

func startRustInteropServer(t *testing.T, url string) (*exec.Cmd, func()) {
	t.Helper()

	_, thisFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatalf("caller: %v", ok)
	}
	repoRoot := filepath.Clean(filepath.Join(filepath.Dir(thisFile), "..", ".."))
	rustDir := filepath.Join(repoRoot, "rust")

	ctx, cancel := context.WithCancel(context.Background())
	cmd := exec.CommandContext(ctx, "cargo", "run", "--example", "interop_echo_server", "--", url)
	cmd.Dir = rustDir
	cmd.Env = append(os.Environ(), "RUST_LOG=error")

	var out bytes.Buffer
	cmd.Stdout = &out
	cmd.Stderr = &out

	if err := cmd.Start(); err != nil {
		cancel()
		t.Fatalf("start rust server: %v", err)
	}

	stop := func() {
		cancel()
		_ = cmd.Process.Signal(os.Interrupt)
		done := make(chan struct{})
		go func() {
			_ = cmd.Wait()
			close(done)
		}()
		select {
		case <-done:
		case <-time.After(2 * time.Second):
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
		}
		if cmd.ProcessState == nil || !cmd.ProcessState.Success() {
			if out.Len() > 0 {
				t.Logf("rust server output:\n%s", out.String())
			}
		}
	}

	return cmd, stop
}
