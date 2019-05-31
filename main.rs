use std::net::{TcpListener,TcpStream};
use std::io::{Write,Read};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::sync::{mpsc::{channel,Sender,Receiver},Mutex,Arc};
use std::collections::BTreeMap;

struct UserInfo {
    name : String,
    input : Receiver<String>,
    output : Sender<String>,
}

fn make_channel_from_stream(stream : TcpStream) -> (Sender<String>,Receiver<String>) {

    let mut input = stream;
    let mut output = input.try_clone().unwrap();

    let mut input = {
            let (send,recv) = channel::<String>();
            rayon::spawn(move || {
                loop {
                    let mut buf = [0;1024];
                    if let Err(err) = input.read(&mut buf) {
                        println!("{:?}",err);
                        return;
                    }

                    if let Err(err) = send.send(String::from_utf8(buf.to_vec()).unwrap()) {
                        println!("{:?}",err);
                        return;
                    }
                }
            });

            recv
        };
    let mut output = {
           let (send,recv) = channel::<String>();

            rayon::spawn(move || {
                loop {
                    let msg = recv.recv().unwrap();
                    let msg = String::as_bytes(&msg);
                    if let Err(err) = output.write(&msg) {
                        println!("{:?}",err);
                        return;
                    }
                }
            });

            send
        };

    (output, input)
}

fn handler(stream : TcpStream, user_list : Arc<Mutex<BTreeMap<String,UserInfo>>>) {
    let mut buf = String::new();
    let mut name = stream.peer_addr().unwrap().to_string();

    let (mut output,mut input) = make_channel_from_stream(stream);

    let mut user_list = user_list.lock().unwrap();

    user_list.insert(name.clone(),UserInfo{
        name,
        input,
        output,
    });
}

fn main() {
    let pool = ThreadPoolBuilder::new()
        .num_threads(8)
        .build()
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:4444").unwrap();

    let user_list = BTreeMap::<String,UserInfo>::new();
    let user_list = Mutex::new(user_list);
    let user_list = Arc::new(user_list);

    println!("server start");

    let user_list_clone = user_list.clone();

    rayon::spawn(move || {
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            let user_list = user_list_clone.clone();

            pool.spawn(move || {
                handler(stream,user_list);
            });
        }
    });

    loop {
        let mut user_list = user_list.lock().unwrap();
        let mut toKill = Vec::new();

        for (_,userInfo) in user_list.iter() {
            if let Ok(msg) = userInfo.input.try_recv() {
                let msg = userInfo.name.clone() + &" : ".to_string() + &msg.clone();
                println!("{}",&msg);


                for (_, userInfo) in user_list.iter() {
                    let msg = msg.clone();
                    if let Err(err) = userInfo.clone().output.send(msg) {
                        toKill.push(userInfo.clone().name.clone());
                    }
                }

            }
        }

        for name in toKill {
            user_list.remove(&name);
        }
    }
}
