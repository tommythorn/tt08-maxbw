![](../../workflows/gds/badge.svg)
![](../../workflows/docs/badge.svg)
![](../../workflows/test/badge.svg)
![](../../workflows/fpga/badge.svg)

# MaxBW

MaxBW is a PCIe/Hypertransport inspired split transaction packetized
memory bus with the following characteristics

* optimal pin usage; all of used for data, none for protocol
* minimal overhead (one byte header for commands and replies)
* fully asynchronous (no fixed latency between commands and replies)
* self-synchronizing (idle channels always transmits aligned idle
  packets)
* supports reply reordering (within a small window)
* credit based flow-control
* address prefix compression

Best case: 64/65 = 98.5% efficient for 64 byte cache loads on byte
aligned channels

Worst case: 25% efficient on byte loads on 32 bit aligned channels

Commands: Idle, Sync, Write(width,addr,data), Read(width,addr)

Replies: Idle(credit), Synced(total_credit), Data(seqdelta,data)

## Packet Transport

   (picture of core and uncore, with a command and reply channel
   between them.  Each has a few packets, some of which are idle.  The
   is an assortment of sizes, examples of address compression, and
   some packet in the reply have replies reordered).

The Packet transport consists of a command and a reply channel.  For
each, it's responsible for detecting the start of a new packet and the
collection of the bits, to be presented to the Packet Protocol layer.

   (picture of a packet, with header broken into fields, followed by
   payload.  Maybe show both Read/Write commands and a Data reply).

A packet is sequence of bytes, the header followed by the payload.
Packets are transmitted on channels with a power-of-two byte width
(typically 1, 2, or 4 bytes).

The packet header encoding, detailed below, encodes the payload length
and the command or reply type respectively.  The reply payload can be
0, 1, 2, 4, 8, 16, 32, or 64 bytes.  Commands further more includes 1,
2, 4, or 8 bytes or address (expect for command 0).

The implementation in this design uses an SDR encoded byte channel for
commands (thus 66 MB/s at 66 MHz) and a DDR encoded 16-bit channel for
replies (thus 66*4=264 MB/s at 66 MHz).

## Packet Protocol

There are four commands: Idle, Sync, Write(size,address,data), and
Read(size,address).  Writes are unacknowledged.  Read command are,
eventually, fulfilled by a Data reply with read data as the payload.
To support reply reordering, Data replies include a small reorder
delta.  Sync is a barrier for all read and write commands which block
until all preceeding commands have been processed.

Replies are: Idle(credits), Synced(total_credits), and
Data(delta,payload).  Synced includes the total credits (in bytes),
Idle include credits returned.  Sender subtracts packet size from his
credits and cannot allow credits to go negative.

Packet size is sum of header byte, length compressed address, length
of data (in the case of Write).


## Reply Header Encoding

Reply headers are 8-bit encoded as two packed fields: `type:5
datasz:3`.  The datasz maps to 0, 1, 2, 4, .., 64 bytes of following
data payload.

XXX This needs to be re-mapped to account for the credits associated
with Idle and Synced replies.

Type is one of Idle, Synced, Data(tagdelta), for tagdelta in -16..15.
As Data isn't valid for a payload of 0, Idle, Synced are mapped onto
the tagdelta.

To enable reply reordering, commands and replies are tagged, but
implicitly to optimize header density.  The first package after a Sync
(Synced) command (reply) is implicitly tagged 0, with sequentially
increasing tags after that.  However, reply tags are formed by
applying the associated offset first.

Example: reply 5 is delayed until after reply 7, thus we'd see .. R3
R4 R6 R7 R5 R8 ... which would be encoded as .. R+0 R+0 R+1 R+1 R-2
R+0 ...  The limited range of tag delta reflects the maximally allowed
reordering.

In summany, the header mapping:

| 8-bit value | meaning                         |
|-------------|---------------------------------|
| 0           | Idle                            |
| 1           | Synced                          |
| ...         |                                 |
| 32          | 1B Data, in-order (tag delta 0) |
| 33          | 1B Data, tag delta 1            |
| ..          |                                 |
| 47          | 1B Data, tag delta 15           |
| 48          | 1B Data, tag delta -16          |
| ...         |                                 |
| 64          | 2B Data, tag delta 0            |
| ..          |                                 |
| 96          | 4B Data, tag delta 0            |
| ..          |                                 |


### Command Headers

Commands are primarily read and write, which both includes (part of)
and address and, for write, a data payload.  Thus, the size encoding
is more complicated.  The header encodes an address size and a data
size, and the payload is the sum of these.

The 8-bit header is broken into three fields `type:3 addrsz:2
datasz:3`.  The datasz mapping is the same as for replies, but addrsz
maps to 1,2,4, or 8 bytes of address, except for type 0 which has no
address bytes and the field is ignored.

The type mapping depends on datasz:

| type, addrsz, datasz | meaning             |
|----------------------|---------------------|
| 0, 0, 0              | Idle                |
| 0, 1, 0              | Sync                |
| n, _, 0              | ReadX, X = 2ⁿ⁻¹  |
| n, _, _              | WriteX, X = 2ⁿ⁻¹ |

### Address Prefix Compression

We allow a full 64-bit address, but we don't want to pay the overhead
of this.  To avoid this, Read and Write commands come in four
different variants: 1B, 2B, 4B, and 8B, where the first three only
sets the lower 8-, 16-, or 32-bits while reusing the rest from the
most recent Read and Write command respectively.  The "previous value"
is reset to 0 by the Sync command and a address is maintained
separately for Read and Write commands to avoid thrashing contexts.

(With a change to instead track last read/written address + 1, we
could introduce a 0 address option which would save a byte for some
commands. It would be a fair bit of additional complexity.)
