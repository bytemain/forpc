package forpc

import "github.com/bytemain/forpc/go/forpc/pb"

const (
	StatusOK                 uint32 = uint32(pb.StatusCode_OK)
	StatusCancelled          uint32 = uint32(pb.StatusCode_CANCELLED)
	StatusUnknown            uint32 = uint32(pb.StatusCode_UNKNOWN)
	StatusInvalidArgument    uint32 = uint32(pb.StatusCode_INVALID_ARGUMENT)
	StatusDeadlineExceeded   uint32 = uint32(pb.StatusCode_DEADLINE_EXCEEDED)
	StatusNotFound           uint32 = uint32(pb.StatusCode_NOT_FOUND)
	StatusAlreadyExists      uint32 = uint32(pb.StatusCode_ALREADY_EXISTS)
	StatusPermissionDenied   uint32 = uint32(pb.StatusCode_PERMISSION_DENIED)
	StatusResourceExhausted  uint32 = uint32(pb.StatusCode_RESOURCE_EXHAUSTED)
	StatusFailedPrecondition uint32 = uint32(pb.StatusCode_FAILED_PRECONDITION)
	StatusAborted            uint32 = uint32(pb.StatusCode_ABORTED)
	StatusOutOfRange         uint32 = uint32(pb.StatusCode_OUT_OF_RANGE)
	StatusUnimplemented      uint32 = uint32(pb.StatusCode_UNIMPLEMENTED)
	StatusInternal           uint32 = uint32(pb.StatusCode_INTERNAL)
	StatusUnavailable        uint32 = uint32(pb.StatusCode_UNAVAILABLE)
	StatusDataLoss           uint32 = uint32(pb.StatusCode_DATA_LOSS)
	StatusUnauthenticated    uint32 = uint32(pb.StatusCode_UNAUTHENTICATED)
)
