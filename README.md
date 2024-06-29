![](../../workflows/gds/badge.svg)
![](../../workflows/docs/badge.svg)
![](../../workflows/test/badge.svg)
![](../../workflows/fpga/badge.svg)

# (Questions still under investigations)

In conflict: separate cmd/type and length, large payloads, dense
encoding, address prefix compression.

Replies and commands have very different needs.  Reply payloads are 0
or 2ⁿ bytes (for n in 0..6).  Command payloads can be quite varied
with 0 to 8+64 = 72 bytes (for a cacheline write on a 64b system).

Without considering address prefix compression and assuming 32-bit
addressing, we have payloads of 0 or 2ⁿ bytes (for n in 0..6) size +
addresses (the width is part of the command).

*WITH* address prefix compression we have to account for different
length addresses -- this is hard but also important

# MaxBW

MaxBW is a PCIe/Hypertransport inspired split transaction packetized
memory bus with the following characteristics

* fully asynchronous (no fixed latency between commands and replies)
* minimal overhead (one byte header for commands and replies)
* self-synchronizing (idle channels always transmits aligned idle
  packets)
* supports reply reordering (within a small window)
* flow-control via pause/resume (AKA Xoff/Xon) replies
* [address prefix compression -- TBD]

Best case: 64/65 = 98.5% efficient for 64 byte cache loads on byte
aligned channels

Worst case: 25% efficient on byte loads on 32 bit aligned channels

Commands: Idle, Sync, Write(width,addr,data), Read(width,addr)

Replies: Idle, Synced, Pause, Resume, Data(seqdelta,data)

## Packet Transport

   (picture of core and uncore, with an ingress and egress channel
   between them.  Each has a few packets, some of which are idle.
   Packet in the egress channel have tags in-order, which as the
   ingress channel have some replies reordered).

The Packet Transport consists of an egress and an ingress channel.
For each, it's responsible for detecting the start of a new packet and
the collection of the bits, to be presented to the Packet Protocol
layer.

   (picture of a packet, with header broken into fields, followed by
   payload.  Maybe show both Read/Write commands and a Data reply).

A packet is sequence of bytes, the header followed by the payload.
Packets are transmitted on channels with a power-of-two byte width
(typically 1, 2, or 4 bytes).

The packet header encoding is still *TBD* but encodes the payload
length and the command or reply type respectively.  The reply payload
can be 0, 1, 2, 4, 8, 16, 32, or 64 bytes.

The implementation in this design uses an SDR encoded byte channel for
commands (thus 66 MB/s at 66 MHz) and a DDR encoded 16-bit channel for
replies (thus 66*4=264 MB/s at 66 MHz).

## Packet Protocol

There are four commands: Idle, Sync, Write(size,address,data), and
Read(size,address).  Write messages are unacknowledged.  Read command
are, eventually, fulfilled by a Data reply with read data as the
payload.  To support reply reordering, Data replies include a small
reorder delta.  Sync is a barrier for all read and write commands
which block until all preceeding commands have been processed.

Replies are: Idle, Synced, Pause, Resume, and Data(delta,payload).
