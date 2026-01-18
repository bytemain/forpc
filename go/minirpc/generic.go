package minirpc

func RegisterUnary[Req any, Resp any](peer *RpcPeer, method string, h func(*Req, map[string]string, *RpcPeer) (*Resp, *RpcError)) {
	peer.Register(method, func(r Request, p *RpcPeer) Response {
		var payload []byte
		if r.Payload != nil {
			payload = r.Payload
		} else if r.Stream != nil {
			for pkt := range r.Stream {
				if pkt.Kind == FrameData {
					payload = pkt.Payload
				}
			}
		}
		if len(payload) == 0 {
			return ResponseError(StatusInvalidArgument, "missing payload")
		}
		var req Req
		if err := p.userUnmarshal(payload, &req); err != nil {
			return ResponseError(StatusInvalidArgument, err.Error())
		}
		resp, rpcErr := h(&req, r.Metadata, p)
		if rpcErr != nil {
			return Response{Metadata: map[string]string{}, Payload: nil, Status: Status{Code: rpcErr.Code, Message: rpcErr.Message}}
		}
		out, err := p.userMarshal(resp)
		if err != nil {
			return ResponseError(StatusInternal, err.Error())
		}
		return ResponseOK(out)
	})
}

func RegisterTypeByNamespace[T any](peer *RpcPeer, namespace string, name string) error {
	var zero T
	return peer.RegisterTypeByNamespace(zero, namespace, name)
}

func CallUnary[Req any, Resp any](peer *RpcPeer, method string, req *Req) (*Resp, error) {
	var resp Resp
	if err := peer.Call(method, req, &resp); err != nil {
		return nil, err
	}
	return &resp, nil
}

func CallUnaryWithMetadata[Req any, Resp any](peer *RpcPeer, method string, req *Req, meta map[string]string) (*Resp, error) {
	var resp Resp
	if err := peer.CallWithMetadata(method, req, meta, &resp); err != nil {
		return nil, err
	}
	return &resp, nil
}

func Stream[Req any, Resp any](peer *RpcPeer, method string) (*BidiStream[Req, Resp], error) {
	return StreamWithMetadata[Req, Resp](peer, method, map[string]string{})
}

func StreamWithMetadata[Req any, Resp any](peer *RpcPeer, method string, meta map[string]string) (*BidiStream[Req, Resp], error) {
	streamID, ch, err := peer.streamInternal(method, meta)
	if err != nil {
		return nil, err
	}
	return &BidiStream[Req, Resp]{streamID: streamID, peer: peer, recvCh: ch}, nil
}
