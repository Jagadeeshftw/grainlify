package soroban

import (
	"fmt"

	"github.com/stellar/go/keypair"
	"github.com/stellar/go/txnbuild"
	"github.com/stellar/go/xdr"
)

// EncodeScValString encodes a string as ScVal
func EncodeScValString(s string) (xdr.ScVal, error) {
	// Convert string to ScSymbol or ScString
	// For now, we'll use ScString
	scStr := xdr.ScString(s)
	return xdr.ScVal{
		Type: xdr.ScValTypeScvString,
		Str:  &scStr,
	}, nil
}

// EncodeScValInt64 encodes an int64 as ScVal
func EncodeScValInt64(i int64) (xdr.ScVal, error) {
	i64 := xdr.Int64(i)
	return xdr.ScVal{
		Type: xdr.ScValTypeScvI64,
		I64:  &i64,
	}, nil
}

// EncodeScValUint64 encodes a uint64 as ScVal
func EncodeScValUint64(u uint64) (xdr.ScVal, error) {
	u64 := xdr.Uint64(u)
	return xdr.ScVal{
		Type: xdr.ScValTypeScvU64,
		U64:  &u64,
	}, nil
}

// EncodeScValAddress encodes an address string as ScVal
func EncodeScValAddress(addrStr string) (xdr.ScVal, error) {
	// Try parsing as account address first
	kp, err := keypair.ParseAddress(addrStr)
	if err == nil {
		// It's an account address
		accountID := kp.Address()
		accountIdXdr, err := xdr.AddressToAccountId(accountID)
		if err != nil {
			return xdr.ScVal{}, fmt.Errorf("failed to convert account address: %w", err)
		}
		scAddr := xdr.ScAddress{
			Type:      xdr.ScAddressTypeScAddressTypeAccount,
			AccountId: &accountIdXdr,
		}
		return xdr.ScVal{
			Type:    xdr.ScValTypeScvAddress,
			Address: &scAddr,
		}, nil
	}

	// Try as contract address (hex or base64)
	contractAddr, err := EncodeContractAddress(addrStr)
	if err == nil {
		return xdr.ScVal{
			Type:    xdr.ScValTypeScvAddress,
			Address: &contractAddr,
		}, nil
	}

	return xdr.ScVal{}, fmt.Errorf("invalid address format: %s", addrStr)
}

// EncodeScValVec encodes a slice of ScVal as ScVal vector
func EncodeScValVec(vals []xdr.ScVal) (xdr.ScVal, error) {
	vec := xdr.ScVec(vals)
	vecPtr := &vec
	return xdr.ScVal{
		Type: xdr.ScValTypeScvVec,
		Vec:  &vecPtr,
	}, nil
}

// EncodeScSymbol encodes a symbol (function name) as ScSymbol
func EncodeScSymbol(s string) (xdr.ScSymbol, error) {
	// ScSymbol is just a string in XDR
	return xdr.ScSymbol(s), nil
}

// BuildInvokeHostFunctionOp builds an InvokeHostFunction operation for contract calls
func BuildInvokeHostFunctionOp(contractAddress xdr.ScAddress, functionName string, args []xdr.ScVal) (txnbuild.Operation, error) {
	symbol, err := EncodeScSymbol(functionName)
	if err != nil {
		return nil, fmt.Errorf("failed to encode function name: %w", err)
	}

	// Build InvokeContractArgs
	invokeArgs := xdr.InvokeContractArgs{
		ContractAddress: contractAddress,
		FunctionName:    symbol,
		Args:            args,
	}

	// Build HostFunction
	hostFunction := xdr.HostFunction{
		Type:           xdr.HostFunctionTypeHostFunctionTypeInvokeContract,
		InvokeContract: &invokeArgs,
	}

	return &txnbuild.InvokeHostFunction{
		HostFunction: hostFunction,
	}, nil
}
