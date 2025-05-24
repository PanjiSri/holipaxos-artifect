package state

import (
	"bytes"
	"encoding/base64"
	"fmt"
	"sync"
)

type Operation uint8

const (
	NONE Operation = iota
	PUT
	GET
	DELETE
	RLOCK
	WLOCK
)

type Value []byte

//const NIL Value = []byte{0}

var NIL = []byte{0}

type Key []byte

func (k *Key) B64() string {
	return string(*k)
}

func KeyFromB64(k string) Key {
	kb, err := base64.StdEncoding.DecodeString(k)

	if err == nil {
		return kb
	}
	return nil
}

func (k Key) ToPeerId(numPeers int32) int32 {
	return (int32(k[0]) + int32(k[len(k)-1])) % numPeers
}

type Command struct {
	ClientId uint32
	OpId     int32
	Op       Operation
	K        Key
	V        Value
}

func (c Command) String() string {
	return fmt.Sprintf("Cmd={K=%v}", c.K)
}

type State struct {
	mutex *sync.Mutex
	Store map[string]Value
	//DB *leveldb.DB
}

func InitState() *State {
	/*
	   d, err := leveldb.Open("/Users/iulian/git/epaxos-batching/dpaxos/bin/db", nil)

	   if err != nil {
	       fmt.Printf("Leveldb open failed: %v\n", err)
	   }

	   return &State{d}
	*/

	return &State{new(sync.Mutex), make(map[string]Value)}
}

func Conflict(gamma *Command, delta *Command) bool {
	//if gamma.K == delta.K {
	if bytes.Equal(gamma.K, delta.K) {
		if gamma.Op == PUT || delta.Op == PUT {
			return true
		}
	}
	return false
}

func ConflictBatch(batch1 []Command, batch2 []Command) bool {
	for i := 0; i < len(batch1); i++ {
		for j := 0; j < len(batch2); j++ {
			if Conflict(&batch1[i], &batch2[j]) {
				return true
			}
		}
	}
	return false
}

func IsRead(command *Command) bool {
	return command.Op == GET
}

func (c *Command) Execute(st *State) Value {
	//fmt.Printf("Executing (%d, %d)\n", c.K, c.V)

	//var key, value [8]byte

	//    st.mutex.Lock()
	//    defer st.mutex.Unlock()

	switch c.Op {
	case PUT:
		/*
		   binary.LittleEndian.PutUint64(key[:], uint64(c.K))
		   binary.LittleEndian.PutUint64(value[:], uint64(c.V))
		   st.DB.Set(key[:], value[:], nil)
		*/

		st.Store[c.K.B64()] = c.V
		return c.V

	case GET:
		if val, present := st.Store[c.K.B64()]; present {
			return val
		}
	}

	return c.V
}
