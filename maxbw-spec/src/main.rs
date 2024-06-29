/*
 * <STOP PRESS>: In the name of entropy (coding density): we are doing
 * Xon/Xoff instead of credits and do _not_ (normally) acknowledge
 * writes.  Also, no tags but reordered results carry relative
 * offsets.  Example:
 *
 *  Cmd: L1 L2 L3 L4 L5  (note, the tag is implicit in the stream)
 *  Rep: D1(0) D2(0) D4(1) D5(0) D3(-2)
 *
 * XXX It does seem a little complicated.
 * </STOP PRESS>
 *
 * We assume bus traffic of commands (and corresponding replies in the
 * reverse direction).  Commands include nop, load1, load2, load4,
 * load8, store1, store2, store4, store8.
 *
 * The challenge of the protocol is balancing transaction density with
 * implementation complexity.  The simplest option is to pack together
 * tag and payload in a bit packet and send those in packets of bytes.
 * There are two options for length encoding: prefixed with a fixed
 * length or lsb encoded, that is, if we are sending 0b1101_1010_1001
 * (12b)
 *
 * Option Length Prefixed (16B): the longest payload is 16B, thus we
 *    use 4b for the length, thus we need 16b = 2B to encode 12b:
 *    <0b0010_1101_1010_1001> => 0x2D, 0xA9
 *    Range: 1B (4b) - 17B (132b)
 * Option Length Prefixed (32B): the longest payload is 32B, thus we
 *    use 5b for the length, thus we need 17b = 3B to encode 12b
 *    <0b0001_0110_1101_01001> => 0x2D, 0xA9
 *    Range: 1B (4b) - 17B (132b)
 * Option LEB: 0b1101_1010_1001 -> 0b1101_1011 0b000_1001_1
 *    Range: 1B (3b) - 17B (119b) - n (n*7b)
 *
 * ... this is incomplete, but it already looks more promising with a
 * length prefixed option.  Question: do we want to support byte
 * granularity for 16b DDR (= 32b) traffic?  Maybe we can require
 * natural padding, eg. 4B for that case and 1B for an 8b stream?




The simplest possible option is a
 * byte-oriented protocol where lsb of each byte but the last is one.
 *
 */


// Assumptions: 16b data transfers, potental alignment, 5b length (in
// 16b), 8b command

#[test]
fn test_des() {
    let nops = [0u16; 32];
    let (deserialized, rest) = deserialize(nops, 2);
    assert_eq!(deserialize.len, 0);
    assert_eq!(deserialize.cmd, Cmd::Nop);
    assert_eq!(rest.len(), 32 - 2);

    // Assumes an alignment of two
    let cmdcmdnopcmd = [// load
			1u16 << 5, 0, 
			// store
			2u16 << 5 + 2, 1, 2, 0,
			// nop
			0u16, 0u16,
			// load

			

    

    


fn main() {
    println!("Hello, world!");
}
