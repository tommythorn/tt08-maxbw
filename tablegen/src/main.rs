#[allow(dead_code)]

/*
Packing 57 commands into 256 opcodes.  Yeah, this work, but isn't very
dense.
*/
fn new_command_decode(h: u8) {
    // header = command:3 addr_size_log2:2 data_size_log2:3
    let data_sz_lg2_p1 = h & 7;
    let addr_sz_lg2 = (h >> 3) & 3;
    let command = (h >> 5) & 7;

    let addr_sz = if command == 0 { 0 } else { 1 << addr_sz_lg2 }; // 1,2,4,8
    let data_sz = if data_sz_lg2_p1 == 0 {
        0
    } else {
        1 << (data_sz_lg2_p1 - 1)
    };
    let _total = data_sz + addr_sz; // 1 .. 72
    let command_p2 = if command == 0 { 0 } else { 1 << (command - 1) };

    if h == 0 {
        assert_eq!(data_sz, 0);
        assert_eq!(addr_sz, 0);
        print!("Idle (A{addr_sz} D{data_sz})");
    } else if h == 1 {
        print!("Sync (A{addr_sz} D{data_sz})");
    } else if command == 0 {
        print!("Reserved (A{addr_sz} D{data_sz})");
    } else if data_sz == 0 && command_p2 != 0 {
        print!("Read{command_p2} (A{addr_sz})");
    } else if command_p2 != 0 && data_sz == command_p2 {
        print!("Write{command_p2} (A{addr_sz} D{data_sz})");
    } else {
        print!("-");
    }
}

#[allow(dead_code)]
fn command_decode(h: u8) {
    // header = D:3 A:2 C:3

    let data_length = if h >> 5 == 0 { 0 } else { 1 << ((h >> 5) - 1) };
    let command = h & 7;
    // XXX this is fail because I forgot I need read16, read32, read64 also
    let addr_length = if h <= 2 { 0 } else { 1 << ((h >> 3) & 3) };
    let total = data_length + addr_length;

    match command {
        0 if data_length == 0 && addr_length == 0 => print!("Idle"),
        1 if data_length == 0 && addr_length == 0 => print!("Sync"),
        3 if data_length != 0 => print!("Write{data_length} (A{addr_length})"),
        4 if data_length == 0 => print!("Read1 (A{addr_length} D{data_length})"),
        5 if data_length == 0 => print!("Read2 (A{addr_length} D{data_length})"),
        6 if data_length == 0 => print!("Read4 (A{addr_length} D{data_length})"),
        7 if data_length == 0 => print!("Read8 (A{addr_length} D{data_length})"),
        _ => print!("-"),
    }
    print!(" [{total}]");
}

#[allow(dead_code)]
fn reply_decode(h: u8) {
    let length = h & 7;
    let typ = h >> 3;

    match (length, typ) {
        (0, 0) => print!("Idle"),
        (0, 1) => print!("Synced"),
        (0, 2) => print!("Pause"),
        (0, 3) => print!("Resume"),
        (0, k) => print!("Invalid {k}"),
        (l, 0) => print!("{}B data, in-order", 1 << (l - 1)),
        (l, td) => print!(
            "{}B data, tag delta {}",
            1 << (l - 1),
            if td < 16 { td as i32 } else { td as i32 - 32 }
        ),
    }
}

fn main() {
    println!("| 8-bit value | meaning                         |");
    println!("|-------------|---------------------------------|");
    for h in 0..=255u8 {
        print!("| {h:3}         | ");
        //reply_decode(h);
        new_command_decode(h);
        println!("|");
    }
}
