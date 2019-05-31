use std::io;
use std::io::{Write,Read};
use std::net::{TcpStream};
use std::sync::mpsc;
use rayon::prelude::*;
use rand::Rng;
use json;

fn gen_name() -> String {
    let charset = b"abcdefghijklmnopqrstuvwxyz";
    let len = 10;

    let mut rng = rand::thread_rng();

    String::from_utf8((0..len)
        .map(|_|{
            charset[rng.gen_range(0,len)]
        })
        .collect()).unwrap()
}

fn main() -> std::io::Result<()>{
    let stdin = io::stdin();
    let mut input = TcpStream::connect("127.0.0.1:4444")?;
    let mut output = input.try_clone()?;

    let mut input = {
           let (send,recv) = mpsc::channel::<String>();

            rayon::spawn(move || {
                loop {
                    let mut buf = [0;1024];
                    input.read(&mut buf).unwrap();

                    send.send(String::from_utf8(buf.to_vec()).unwrap());
                }
            });

            recv
        };
    let mut output = {
           let (send,recv) = mpsc::channel::<String>();

            rayon::spawn(move || {
                loop {
                    let msg = recv.recv().unwrap();
                    let msg = String::as_bytes(&msg);
                    output.write(&msg).unwrap();
                }
            });

            send
        };

    rayon::join(
        move || {
            loop {
                let mut msg = String::new();
                stdin.read_line(&mut msg).unwrap();

                output.send(msg).unwrap();
            }
        },
        move || {
            loop {
                let msg = input.recv().unwrap();

                println!("server : {}",msg);
            }
        }
    );

    Ok(())
}