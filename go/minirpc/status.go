package minirpc

const (
	StatusOK                 uint32 = 0
	StatusCancelled          uint32 = 1
	StatusUnknown            uint32 = 2
	StatusInvalidArgument    uint32 = 3
	StatusDeadlineExceeded   uint32 = 4
	StatusNotFound           uint32 = 5
	StatusAlreadyExists      uint32 = 6
	StatusPermissionDenied   uint32 = 7
	StatusResourceExhausted  uint32 = 8
	StatusFailedPrecondition uint32 = 9
	StatusAborted            uint32 = 10
	StatusOutOfRange         uint32 = 11
	StatusUnimplemented      uint32 = 12
	StatusInternal           uint32 = 13
	StatusUnavailable        uint32 = 14
	StatusDataLoss           uint32 = 15
	StatusUnauthenticated    uint32 = 16
)

