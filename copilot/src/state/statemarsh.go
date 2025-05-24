package state

import (
	"io"
	"log"
)

const (
	KEY_SIZE   = 23
	VALUE_SIZE = 500
)

func (t *Command) Marshal(w io.Writer) {
	var b [VALUE_SIZE]byte

	// ClientId
	bs := b[:4]
	utmp32 := t.ClientId
	bs[0] = byte(utmp32)
	bs[1] = byte(utmp32 >> 8)
	bs[2] = byte(utmp32 >> 16)
	bs[3] = byte(utmp32 >> 24)
	w.Write(bs)

	// OpId
	bs = b[:4]
	tmp32 := t.OpId
	bs[0] = byte(tmp32)
	bs[1] = byte(tmp32 >> 8)
	bs[2] = byte(tmp32 >> 16)
	bs[3] = byte(tmp32 >> 24)
	w.Write(bs)
	// Op
	bs = b[:1]
	b[0] = byte(t.Op)
	w.Write(bs)

	// K
	//binary.LittleEndian.PutUint64(bs, t.K)
	key := []byte(t.K)
	w.Write(key)
	// V
	//binary.LittleEndian.PutUint64(bs, uint64(t.V))
	val := []byte(t.V)
	w.Write(val)
}

func (t *Command) Unmarshal(r io.Reader) error {
	var b [VALUE_SIZE]byte
	bs := b[:4]

	// ClientId
	if _, err := io.ReadAtLeast(r, bs, 4); err != nil {
		return err
	}
	t.ClientId = uint32((uint32(bs[0]) | (uint32(bs[1]) << 8) | (uint32(bs[2]) << 16) | (uint32(bs[3]) << 24)))
	// OpId
	bs = b[:4]
	if _, err := io.ReadAtLeast(r, bs, 4); err != nil {
		return err
	}
	//t.OpId = OperationId((uint32(bs[0]) | (uint32(bs[1]) << 8) | (uint32(bs[2]) << 16) | (uint32(bs[3]) << 24)))
	t.OpId = int32((uint32(bs[0]) | (uint32(bs[1]) << 8) | (uint32(bs[2]) << 16) | (uint32(bs[3]) << 24)))
	// Op
	bs = b[:1]
	if _, err := io.ReadFull(r, bs); err != nil {
		return err
	}
	t.Op = Operation(b[0])
	// K
	bs = b[:KEY_SIZE]
	if _, err := io.ReadFull(r, bs); err != nil {
		return err
	}
	t.K = make([]byte, KEY_SIZE)
	copy(t.K, bs[:KEY_SIZE])
	// V
	bs = b[:VALUE_SIZE]
	if _, err := io.ReadFull(r, bs); err != nil {
		return err
	}
	t.V = bs
	return nil
}

func (t *Key) Marshal(w io.Writer) {
	n, err := w.Write(*t)
	if err != nil {
		log.Println("Error writing key err", err)
	}
	if n != KEY_SIZE {
		log.Println("Error writing key", KEY_SIZE)
	}
}

func (t *Value) Marshal(w io.Writer) {
	n, err := w.Write(*t)
	if err != nil {
		log.Println("Error writing value err", err)
	}
	if n != VALUE_SIZE {
		log.Println("Error writing value", n)
	}
}

func (t *Key) Unmarshal(r io.Reader) error {
	var b [KEY_SIZE]byte
	bs := b[:KEY_SIZE]
	if _, err := io.ReadFull(r, bs); err != nil {
		return err
	}
	*t = bs
	return nil
}

func (t *Value) Unmarshal(r io.Reader) error {
	var b [VALUE_SIZE]byte
	bs := b[:VALUE_SIZE]
	if _, err := io.ReadFull(r, bs); err != nil {
		return err
	}
	*t = bs
	return nil
}
