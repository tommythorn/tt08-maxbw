mod encoding;

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

enum Command {
    Idle,
    Sync,
    Write(u8, u64, Vec<u8>), // XXX Width can only be 1, 2, 4, ..., 64; we could enforce with types
    Read(u8, u64),           // XXX Width can only be 1, 2, 4, ..., 64; we could enforce with types
}

enum Reply {
    Idle,
    Synced,
    Pause,
    Resume,
    Data(i8, Vec<u8>),
}

fn main() {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (reply_tx, reply_rx) = mpsc::channel();

    // Let's spawn the client (~ CPU)
    thread::spawn(move || run_client(cmd_tx, reply_rx));
    run_memory_server(cmd_rx, reply_tx);
}

fn run_client(cmd_tx: Sender<String>, reply_rx: Receiver<Reply>) {
    let mut pending = VecDeque::new();
    let mut tag = 0;
    let mut paused = false;

    let mut n = 1;
    loop {
        if n < 10 && !paused {
            cmd_tx.send(format! {"LOAD #{n}"}).unwrap();
            pending.push_front(n);
            n += 1;
        } else {
            cmd_tx.send("stop".into()).unwrap();
        }

        // Handing any replies coming back
        for reply in reply_rx.try_iter() {
            match reply {
                Reply::Idle => {}
                Reply::Synced => {
                    pending = VecDeque::new(); // XXX flush method?
                    tag = 0;
                    paused = false;
                }
                Reply::Pause => {
                    paused = true;
                }
                Reply::Resume => {
                    paused = false;
                }
                Reply::Data(delta, data) => {
                    let this = tag + delta as i32;
                    println!("Client: got #{this} {data:?}");
                    tag += 1;
                }
            }
            println!("Client: got DATA for {}", pending.pop_back().unwrap());
        }
        thread::sleep(std::time::Duration::from_millis(100)); // block for 0.1 seconds
    }
}

fn run_memory_server(cmd_rx: Receiver<String>, reply_tx: Sender<Reply>) {
    let mut tag = 0;
    //let mut pending_loads = VecDeque::new();

    loop {
        let received = cmd_rx.recv().unwrap();
        println!("Server: got {received}");
        if received == "stop" {
            break;
        }
        reply_tx.send(Reply::Data(0, vec![0u8])).unwrap();
        tag += 1;
    }
}
