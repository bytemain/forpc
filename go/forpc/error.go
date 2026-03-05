package forpc

import (
	"fmt"

	"github.com/bytemain/forpc/go/forpc/pb"
)

type RpcError struct {
	Code    pb.StatusCode
	Message string
}

func (e *RpcError) Error() string {
	return fmt.Sprintf("RpcError{code=%d,message=%q}", e.Code, e.Message)
}

func NewRpcError(code pb.StatusCode, message string) *RpcError {
	return &RpcError{Code: code, Message: message}
}
