package soroban

import (
	"testing"

	"github.com/stellar/go/xdr"
)

func TestEncodeScValString(t *testing.T) {
	val, err := EncodeScValString("test")
	if err != nil {
		t.Fatalf("EncodeScValString failed: %v", err)
	}
	if val.Type != xdr.ScValTypeScvString {
		t.Errorf("expected ScvString, got %v", val.Type)
	}
	if val.Str == nil || *val.Str != "test" {
		t.Errorf("expected 'test', got %v", val.Str)
	}
}

func TestEncodeScValInt64(t *testing.T) {
	val, err := EncodeScValInt64(12345)
	if err != nil {
		t.Fatalf("EncodeScValInt64 failed: %v", err)
	}
	if val.Type != xdr.ScValTypeScvI64 {
		t.Errorf("expected ScvI64, got %v", val.Type)
	}
	if val.I64 == nil || *val.I64 != 12345 {
		t.Errorf("expected 12345, got %v", val.I64)
	}
}

func TestEncodeScValUint64(t *testing.T) {
	val, err := EncodeScValUint64(67890)
	if err != nil {
		t.Fatalf("EncodeScValUint64 failed: %v", err)
	}
	if val.Type != xdr.ScValTypeScvU64 {
		t.Errorf("expected ScvU64, got %v", val.Type)
	}
	if val.U64 == nil || *val.U64 != 67890 {
		t.Errorf("expected 67890, got %v", val.U64)
	}
}

func TestEncodeScValVec(t *testing.T) {
	vals := []xdr.ScVal{
		{Type: xdr.ScValTypeScvI64, I64: func() *xdr.Int64 { v := xdr.Int64(1); return &v }()},
		{Type: xdr.ScValTypeScvI64, I64: func() *xdr.Int64 { v := xdr.Int64(2); return &v }()},
	}
	
	vecVal, err := EncodeScValVec(vals)
	if err != nil {
		t.Fatalf("EncodeScValVec failed: %v", err)
	}
	if vecVal.Type != xdr.ScValTypeScvVec {
		t.Errorf("expected ScvVec, got %v", vecVal.Type)
	}
	if vecVal.Vec == nil {
		t.Fatal("expected non-nil Vec")
	}
	if len(**vecVal.Vec) != 2 {
		t.Errorf("expected vec length 2, got %d", len(**vecVal.Vec))
	}
}

func TestEncodeScSymbol(t *testing.T) {
	symbol, err := EncodeScSymbol("test_function")
	if err != nil {
		t.Fatalf("EncodeScSymbol failed: %v", err)
	}
	if symbol != "test_function" {
		t.Errorf("expected 'test_function', got %v", symbol)
	}
}

func TestEncodeContractAddress(t *testing.T) {
	// Test with hex string (64 chars = 32 bytes)
	hexID := "0000000000000000000000000000000000000000000000000000000000000000"
	addr, err := EncodeContractAddress(hexID)
	if err != nil {
		t.Fatalf("EncodeContractAddress failed with hex: %v", err)
	}
	if addr.Type != xdr.ScAddressTypeScAddressTypeContract {
		t.Errorf("expected Contract type, got %v", addr.Type)
	}
	if addr.ContractId == nil {
		t.Fatal("expected non-nil ContractId")
	}
}

func TestDefaultRetryConfig(t *testing.T) {
	config := DefaultRetryConfig()
	if config.MaxRetries != 3 {
		t.Errorf("expected MaxRetries 3, got %d", config.MaxRetries)
	}
	if config.InitialDelay.Seconds() != 1 {
		t.Errorf("expected InitialDelay 1s, got %v", config.InitialDelay)
	}
	if config.MaxDelay.Seconds() != 30 {
		t.Errorf("expected MaxDelay 30s, got %v", config.MaxDelay)
	}
	if config.BackoffMultiplier != 2.0 {
		t.Errorf("expected BackoffMultiplier 2.0, got %f", config.BackoffMultiplier)
	}
}
