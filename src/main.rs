extern crate mio;
use std::collections::HashMap;
use std::io::*;
use mio::net::{TcpListener, TcpStream};
use mio::{Token, Poll, Ready, PollOpt, Events};

fn main() {

    // 服务端fd的标志
    const SERVER: Token = Token(0);
    let mut curr_client_id = 1;

    // 服务端监听的地址端口
    let tcp_addr = "127.0.0.1:9000".parse().unwrap();

    // 创建tcp监听端口到 tcp_addr
    let tcp_server = TcpListener::bind(&tcp_addr).unwrap();

    // 创建一个poll对象
    let poll = Poll::new().unwrap();

    let pollopts = if cfg!(feature = "level") {
        PollOpt::level()
    } else {
        if cfg!(feature = "oneshot") {
            PollOpt::edge() | PollOpt::oneshot()
        } else {
            PollOpt::edge()
        }
    };

    // 将服务端fd注册到poll中做读事件监听
    poll.register(&tcp_server, SERVER, Ready::readable(), PollOpt::edge())
        .unwrap();

    let mut events = Events::with_capacity(1024); // 事件对象数组
    let buf = &mut [0u8; 1024];      // 缓存数据的buf
    let mut streams = HashMap::new();//只要流保存了，TCP连接就不关闭，根据客户端的id取流
    loop {
        // 获取当前事件列表，类似于 epoll_wait
        poll.poll(&mut events, None).unwrap();

        // 遍历事件列表，并处理事件
        for event in events.iter() {
            // 根据注册到poll中时的token来判断是监听fd还是连接fd
            match event.token() {
                SERVER => {
                    // 调用 accept 接收一个客户端连接
                    let (stream, _addr) = tcp_server.accept().unwrap();
                    let client_id = curr_client_id;
                    curr_client_id += 1;
                    let token = Token(client_id);

                    if let Some(_stream) = streams.insert(client_id, stream) {
                        panic!("Stream entry token filled.");
                    }

                    // 将新的连接fd注册到poll中，关注其读事件
                    poll.register(&streams[&client_id], token, Ready::readable(), pollopts)
                        .unwrap();//为每一个新的TCP 客户端注册不同的token
                }
                Token(client_id) => {
                    // 收到来自客户端的消息，并做echo处理
                    let mut need_close = false;
                    if event.readiness().is_readable() {
                        let stream = streams.get_mut(&client_id).unwrap();
                        match stream.read(buf) {
                            Ok(nread) => {
                                let tmp_str = std::str::from_utf8(buf).unwrap().trim();
                                print!("recv: {}", tmp_str);
                                //println!("Received  {} Bytes", nread);
                                // 如果收到的字节数为0，则标明对端关闭连接
                                if nread == 0 {
                                    need_close = true;
                                }
                                else 
                                {
                                    match stream.write(&buf[..nread]) {
                                        Ok(nwrite) => println!("write  {} bytes", nwrite),
                                        Err(e) => println!("write err  {}", e),
                                    }
                                }
                            }
                            Err(e) => println!("read err  {}", e),
                        }
                    }
                    
                    // 如果对端关闭连接，则本端也将tcp流从map中删掉，完成资源的释放
                    if need_close {
                        streams.remove(&client_id).unwrap();
                        println!("client_id {} is removed", client_id);
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}
