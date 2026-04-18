package forpc

import (
	"google.golang.org/protobuf/proto"

	"github.com/bytemain/forpc/go/forpc/pb"
)

// Packet is the protobuf-defined wire packet (alias of pb.Packet) used by the
// peer to multiplex streams over a single transport. The wire format is
// produced and consumed entirely via protobuf encoding; there is no
// hand-written framing on top.
type Packet = pb.Packet

// FrameKind constants mirror the pb.FrameKind enum and are exposed under
// shorter names for convenience at call sites.
const (
	FrameHeaders   = pb.FrameKind_HEADERS
	FrameData      = pb.FrameKind_DATA
	FrameTrailers  = pb.FrameKind_TRAILERS
	FrameRstStream = pb.FrameKind_RST_STREAM
)

// EncodePacket marshals a Packet to its protobuf wire bytes.
func EncodePacket(p *Packet) ([]byte, error) {
	return proto.Marshal(p)
}

// DecodePacket parses a Packet from its protobuf wire bytes.
func DecodePacket(b []byte) (*Packet, error) {
	p := &Packet{}
	if err := proto.Unmarshal(b, p); err != nil {
		return nil, err
	}
	return p, nil
}

// HeadersPacket builds a HEADERS frame carrying an encoded Call.
func HeadersPacket(streamID uint32, call *pb.Call) (*Packet, error) {
	payload, err := proto.Marshal(call)
	if err != nil {
		return nil, err
	}
	return &Packet{StreamId: streamID, Kind: FrameHeaders, Payload: payload}, nil
}

// DataPacket builds a DATA frame carrying user payload bytes.
func DataPacket(streamID uint32, payload []byte) *Packet {
	return &Packet{StreamId: streamID, Kind: FrameData, Payload: payload}
}

// TrailersPacket builds a TRAILERS frame carrying an encoded Status.
func TrailersPacket(streamID uint32, status *pb.Status) (*Packet, error) {
	payload, err := proto.Marshal(status)
	if err != nil {
		return nil, err
	}
	return &Packet{StreamId: streamID, Kind: FrameTrailers, Payload: payload}, nil
}

// RstStreamPacket builds a RST_STREAM frame with the given error code.
func RstStreamPacket(streamID uint32, errorCode uint32) *Packet {
	return &Packet{StreamId: streamID, Kind: FrameRstStream, ErrorCode: errorCode}
}

// Call is a peer-side view of a pb.Call decoded from a HEADERS frame.
type Call struct {
	Method   string
	Metadata map[string]string
}

// Status is a peer-side view of a pb.Status decoded from a TRAILERS frame.
type Status struct {
	Code    pb.StatusCode
	Message string
}

func StatusOKValue() Status {
	return Status{Code: pb.StatusCode_OK, Message: "OK"}
}

func (s Status) IsOK() bool {
	return s.Code == pb.StatusCode_OK
}
