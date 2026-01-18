package minirpc

import "fmt"

type RpcError struct {
	Code    uint32
	Message string
}

func (e *RpcError) Error() string {
	return fmt.Sprintf("RpcError{code=%d,message=%q}", e.Code, e.Message)
}

func NewRpcError(code uint32, message string) *RpcError {
	return &RpcError{Code: code, Message: message}
}

