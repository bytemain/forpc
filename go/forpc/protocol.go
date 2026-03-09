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

// EncodeBatch encodes multiple packets into a single buffer.
// If there is only one packet it is encoded normally (no batch overhead).
// Otherwise the batch format uses stream_id=0 as a sentinel:
//
//	[0x00000000: u32 BE]  -- batch sentinel
//	[count: u32 BE]       -- number of sub-packets
//	For each sub-packet:
//	  [len: u32 BE]       -- length of encoded packet bytes
//	  [packet bytes...]   -- standard packet format
func EncodeBatch(packets []Packet) ([]byte, error) {
	if len(packets) == 1 {
		return packets[0].Encode()
	}
	encoded := make([][]byte, len(packets))
	total := 8 // sentinel + count
	for i, pkt := range packets {
		b, err := pkt.Encode()
		if err != nil {
			return nil, err
		}
		encoded[i] = b
		total += 4 + len(b) // length prefix + packet bytes
	}
	out := make([]byte, total)
	binary.BigEndian.PutUint32(out[0:4], 0) // sentinel
	binary.BigEndian.PutUint32(out[4:8], uint32(len(packets)))
	offset := 8
	for _, enc := range encoded {
		binary.BigEndian.PutUint32(out[offset:offset+4], uint32(len(enc)))
		offset += 4
		copy(out[offset:], enc)
		offset += len(enc)
	}
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

// DecodePackets decodes a buffer that may contain a single packet or a batch.
// If the first 4 bytes are 0x00000000 (stream_id=0) it is treated as a batch
// envelope. If batch decoding fails (e.g. a regular packet with stream_id=0),
// it falls back to single-packet decoding.
func DecodePackets(b []byte) ([]Packet, error) {
	if len(b) < 5 {
		return nil, errors.New("packet too short")
	}
	streamID := binary.BigEndian.Uint32(b[:4])
	if streamID == 0 {
		if packets, err := tryDecodeBatch(b); err == nil {
			return packets, nil
		}
	}
	pkt, err := DecodePacket(b)
	if err != nil {
		return nil, err
	}
	return []Packet{pkt}, nil
}

func tryDecodeBatch(b []byte) ([]Packet, error) {
	if len(b) < 8 {
		return nil, errors.New("batch too short")
	}
	count := binary.BigEndian.Uint32(b[4:8])
	if count < 2 {
		return nil, errors.New("not a batch")
	}
	packets := make([]Packet, 0, count)
	offset := 8
	for i := uint32(0); i < count; i++ {
		if offset+4 > len(b) {
			return nil, errors.New("batch truncated")
		}
		pktLen := int(binary.BigEndian.Uint32(b[offset : offset+4]))
		offset += 4
		if pktLen < 5 || offset+pktLen > len(b) {
			return nil, errors.New("batch sub-packet truncated")
		}
		pkt, err := DecodePacket(b[offset : offset+pktLen])
		if err != nil {
			return nil, err
		}
		packets = append(packets, pkt)
		offset += pktLen
	}
	return packets, nil
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
