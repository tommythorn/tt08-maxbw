//! Model of the MaxBW protocol
//!
//! Todo:
//! - Flow control (to avoid overrunning buffer space and delta range)
//! - Message serialization
//! - Better code structure + unit tests

//mod encoding;
use rand::Rng;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

const WINDOW_SIZE: usize = 16;

struct Endpoint<S: std::fmt::Debug, R: std::fmt::Debug> {
    tx: Sender<S>,
    rx: Receiver<R>,
    id: String,
}

impl<S: std::fmt::Debug, R: std::fmt::Debug> Endpoint<S, R> {
    fn new(tx: Sender<S>, rx: Receiver<R>, id: String) -> Self {
        Self { tx, rx, id }
    }

    fn send(&self, msg: S) {
        println!("{}{msg:?} ->", self.id);
        self.tx.send(msg).unwrap();
    }

    fn receive(&self) -> R {
        let v = self.rx.recv().unwrap();
        println!("{} <- {v:?}", self.id);
        v
    }
}

#[derive(Debug)]
enum Command {
    Idle,
    Sync,
    Write(u64, Vec<u8>), // XXX Width can only be 1, 2, 4, ..., 64; we could enforce with types
    Read(u8, u64),       // XXX Width can only be 1, 2, 4, ..., 64; we could enforce with types
    EndSim,              // Sim-only
}

#[derive(Debug)]
enum Reply {
    Idle(u8),
    Synced(u8),
    Data(i8, Vec<u8>),
}

fn main() {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (reply_tx, reply_rx) = mpsc::channel();
    let client_channels = Endpoint::new(cmd_tx, reply_rx, "client: ".into());
    let server_channels = Endpoint::new(reply_tx, cmd_rx, "\t\tserver: ".into());
    thread::spawn(move || memory_server(server_channels));
    client(client_channels);
}

fn client(ep: Endpoint<Command, Reply>) {
    let mut pending_reads = vec![None; WINDOW_SIZE];
    let mut pending_reads_count = 0;
    let mut oldest_read_tag = 0;
    let mut next_read_tag = 0;
    let mut next_data_tag = 0;
    let mut available_credits; // Note, doesn't apply to Idle and Sync

    // Sending idle isn't required but reflects what would happen on hardware
    ep.send(Command::Idle);

    // We need to start out in a known state
    ep.send(Command::Sync);
    loop {
        match ep.receive() {
            Reply::Synced(credits) => {
                available_credits = credits;
                break;
            }
            _ => {}
        }
    }

    println!();

    let mut n = 1;
    loop {
        //println!("client: C {available_credits}");
        let cmd = if n < 100 && next_read_tag - oldest_read_tag < WINDOW_SIZE {
            let is_read = rand::thread_rng().gen_range(0..100) < 66;
            let magic_number = rand::thread_rng().gen_range(1..6);
            let length = 1 << (magic_number / 2);
            let a = rand::thread_rng().gen_range(1..1000u64);
            if is_read {
                /* XXX compressed address */
                if available_credits < 1 + 8 {
                    Command::Idle
                } else {
                    assert!(pending_reads[next_read_tag % WINDOW_SIZE].is_none()); // Can't happen
                    pending_reads[next_read_tag % WINDOW_SIZE] = Some(a);
                    next_read_tag += 1;
                    pending_reads_count += 1;
                    available_credits -= 1 + 8;
                    n += 1;
                    Command::Read(length, a)
                }
            } else {
                /* XXX compressed address */
                if available_credits < length + 1 + 8 {
                    Command::Idle
                } else {
                    available_credits -= length + 1 + 8;
                    Command::Write(a, vec![1u8; length.into()])
                }
            }
        } else if pending_reads_count == 0 {
            Command::EndSim
        } else {
            Command::Idle
        };

        ep.send(cmd);

        // Handle replies
        match ep.receive() {
            Reply::Idle(back_credits) => available_credits += back_credits,
            Reply::Synced(credits) => {
                available_credits = credits;
                pending_reads = vec![None; WINDOW_SIZE];
                pending_reads_count = 0;
                oldest_read_tag = 0;
                next_read_tag = 0;
                next_data_tag = 0;
            }
            Reply::Data(delta, data) => {
                let this = next_data_tag + delta as i32;
                match pending_reads[this as usize % WINDOW_SIZE] {
                    None => panic!(
                        "client: got data for unknown read #{this} (delta {delta}, next_data_tag {next_data_tag}"
                    ),
                    Some(a) => {
                        println!(
                            "client: read #{this} {data:?} address {a} (delta {})",
                            this - next_data_tag
                        );
                        pending_reads[this as usize % WINDOW_SIZE] = None;
                        pending_reads_count -= 1;
                        while oldest_read_tag < next_read_tag
                            && pending_reads[oldest_read_tag % WINDOW_SIZE].is_none()
                        {
                            oldest_read_tag += 1;
                        }
                    }
                }
                next_data_tag += 1;
            }
        }
    }
}

fn memory_server(ep: Endpoint<Reply, Command>) {
    let mut pending_reads = vec![None; WINDOW_SIZE];
    let mut pending_reads_count = 0;
    let mut oldest_read_tag = 0;
    let mut next_read_tag = 0;
    let mut next_data_tag = 0;
    let mut free_credits = 128usize; // XXX Whatever the buffer size is
    let mut back_credits = 0;

    // Sending idle isn't required but reflects what would happen on hardware
    ep.send(Reply::Idle(0));

    'outer: loop {
        // XXX Simple model of credits as replentishing 3 per cycle
        if back_credits + free_credits < 128 {
            back_credits += 3;
        }

        // Handle new commands
        match ep.receive() {
            Command::Idle => {}
            Command::Sync => {
                pending_reads = vec![None; WINDOW_SIZE];
                pending_reads_count = 0;
                oldest_read_tag = 0;
                next_read_tag = 0;
                next_data_tag = 0;
                ep.send(Reply::Synced(free_credits as u8));
            }
            Command::Write(_a, d) => {
                assert!(1 + 8 + d.len() <= free_credits);
                free_credits -= 1 + 8 + d.len();
            }
            Command::Read(width, addr) => {
                assert!(1 + 8 <= free_credits);
                free_credits -= 1 + 8;
                assert!(next_data_tag - oldest_read_tag != WINDOW_SIZE); // XXX Window overflowing is a flow-control failure
                assert!(pending_reads[next_read_tag % WINDOW_SIZE].is_none()); // Can't happen
                pending_reads[next_read_tag % WINDOW_SIZE] = Some((width, addr));
                next_read_tag += 1;
                pending_reads_count += 1;
            }
            Command::EndSim => return,
        }

        // Service pending reads
        if 0 < pending_reads_count && rand::thread_rng().gen_range(0..100) < 20 {
            let target_index = rand::thread_rng().gen_range(0..pending_reads_count);
            let mut i = target_index;

            // XXX I'm sure there's a better way
            for j in 0..WINDOW_SIZE {
                let target_tag = j + oldest_read_tag;
                if pending_reads[target_tag % WINDOW_SIZE].is_some() {
                    if 0 < i {
                        i -= 1;
                    } else {
                        let (width, _addr) = pending_reads[target_tag % WINDOW_SIZE].unwrap();
                        pending_reads[target_tag % WINDOW_SIZE] = None;
                        let delta = target_tag as isize - next_data_tag as isize;
                        assert_eq!(delta, delta as i8 as isize); // Should be impossible
                        ep.send(Reply::Data(delta as i8, vec![0u8; width as usize]));
                        next_data_tag += 1;
                        pending_reads_count -= 1;

                        // Make room as the oldest drop out of the window
                        while oldest_read_tag < next_read_tag
                            && pending_reads[oldest_read_tag % WINDOW_SIZE].is_none()
                        {
                            oldest_read_tag += 1;
                        }
                        continue 'outer;
                    }
                }
            }
            panic!("Found no reads? {pending_reads:?} {target_index} {oldest_read_tag}");
        } else {
            ep.send(Reply::Idle(back_credits as u8));
            free_credits += back_credits;
            back_credits = 0;
        }
    }
}
