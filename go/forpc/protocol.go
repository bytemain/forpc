package forpc

import (
	"encoding/binary"
	"errors"

	"github.com/bytemain/forpc/go/forpc/pb"
)

const (
	FrameHeaders   uint8 = 0
	FrameData      uint8 = 1
	FrameTrailers  uint8 = 2
	FrameRstStream uint8 = 3
)

type Packet struct {
	StreamID uint32
	Kind     uint8
	Payload  []byte
}

func (p Packet) Encode() ([]byte, error) {
	out := make([]byte, 5+len(p.Payload))
	binary.BigEndian.PutUint32(out[:4], p.StreamID)
	out[4] = p.Kind
	copy(out[5:], p.Payload)
	return out, nil
}

func DecodePacket(b []byte) (Packet, error) {
	if len(b) < 5 {
		return Packet{}, errors.New("packet too short")
	}
	streamID := binary.BigEndian.Uint32(b[:4])
	kind := b[4]
	payload := make([]byte, len(b)-5)
	copy(payload, b[5:])
	return Packet{StreamID: streamID, Kind: kind, Payload: payload}, nil
}

type Call struct {
	Method   string
	Metadata map[string]string
}

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
