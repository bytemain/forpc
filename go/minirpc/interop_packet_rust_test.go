//go:build integration

package minirpc

import (
	"bytes"
	"context"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"testing"
	"time"

	mangostransport "github.com/bytemain/mini-rpc/go/transport/mangos"
)

func TestInteropRustGoPacketEcho(t *testing.T) {
	url := pickTCPURL(t)

	cmd, stop := startRustPacketEchoServer(t, url)
	defer stop()
	_ = cmd

	dealer, err := mangostransport.DialWithRetry(url, 50, 100*time.Millisecond)
	if err != nil {
		t.Fatalf("dial: %v", err)
	}
	defer dealer.Close()

	in := Packet{StreamID: 1, Kind: FrameData, Payload: []byte("Hello")}
	b, _ := in.Encode()
	if _, err := dealer.Send(b); err != nil {
		t.Fatalf("send: %v", err)
	}
	msg, err := dealer.Recv()
	if err != nil {
		t.Fatalf("recv: %v", err)
	}
	defer msg.Free()

	out, err := DecodePacket(msg.Body)
	if err != nil {
		t.Fatalf("decode: %v", err)
	}
	if out.StreamID != 1 || out.Kind != FrameData || string(out.Payload) != "Hello" {
		t.Fatalf("unexpected packet: %#v", out)
	}
}

func startRustPacketEchoServer(t *testing.T, url string) (*exec.Cmd, func()) {
	t.Helper()

	_, thisFile, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatalf("caller: %v", ok)
	}
	repoRoot := filepath.Clean(filepath.Join(filepath.Dir(thisFile), "..", ".."))
	rustDir := filepath.Join(repoRoot, "rust")

	ctx, cancel := context.WithCancel(context.Background())
	cmd := exec.CommandContext(ctx, "cargo", "run", "--example", "interop_packet_echo_server", "--", url)
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
		if out.Len() > 0 {
			t.Logf("rust server output:\n%s", out.String())
		}
	}

	deadline := time.Now().Add(10 * time.Second)
	for time.Now().Before(deadline) {
		addr := url[len("tcp://"):]
		conn, err := net.DialTimeout("tcp", addr, 100*time.Millisecond)
		if err == nil {
			_ = conn.Close()
			break
		}
		time.Sleep(50 * time.Millisecond)
	}

	return cmd, stop
}
