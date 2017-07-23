extern crate zmq;

use std::io::{self, BufRead};

enum ConnectOrBind {
    Connect,
    Bind,
}

struct Service {
    separator: Option<String>,
    connect_or_bind: ConnectOrBind,
    address: String,
}

struct Sub {
    service: Service,
    filter: String,
}

struct Proxy {
    frontend_kind: zmq::SocketType,
    frontend_address: String,
    backend_kind: zmq::SocketType,
    backend_address: String,
}

enum CmdlineArgs {
    Pub(Service),
    Sub(Sub),
    Req(Service),
    Rep(Service),
    Proxy(Proxy),
}

fn connect_or_bind_of_str(s: &str) -> Result<ConnectOrBind, String> {
    match s {
        "connect" => Ok(ConnectOrBind::Connect),
        "bind" => Ok(ConnectOrBind::Bind),
        _ => Err(format!("Expected 'connect' or 'bind' but got {}", s)),
    }
}

fn socket_type_of_str(s: &str) -> Result<zmq::SocketType, String> {
    match s {
        "router" => Ok(zmq::ROUTER),
        "dealer" => Ok(zmq::DEALER),
        "xsub" => Ok(zmq::XSUB),
        "xpub" => Ok(zmq::XPUB),
        _ => Err(format!("Unknown socket kind {}", s)),
    }
}

fn parse_service_args(args: &mut std::env::Args) -> Result<Service, String> {
    let arg = &args.next().ok_or("Missing connect or bind or --multi-message")?;
    let (separator, connect_or_bind) =
        if arg == "--multi-message" {
            (Some(args.next().ok_or("Missing separator")?),
             connect_or_bind_of_str(
                 &args.next().ok_or("Missing connect or bind")?)?)
        }
        else {
            (None, connect_or_bind_of_str(arg)?)
        };
    Ok (Service {
        separator: separator,
        connect_or_bind: connect_or_bind,
        address: args.next().ok_or("Missing address")?,
    })
}

fn parse_sub_args(mut args: &mut std::env::Args) -> Result<Sub, String> {
    Ok(Sub {
        service: parse_service_args(&mut args)?,
        filter: args.next().ok_or("Missing filter")?,
    })
}

fn parse_proxy_args(args: &mut std::env::Args) -> Result<Proxy, String> {
    Ok(Proxy {
        frontend_kind: socket_type_of_str(
            &args.next().ok_or("Missing frontend kind")?)?,
        frontend_address:
            args.next().ok_or("Missing frontend address")?,
        backend_kind: socket_type_of_str(
            &args.next().ok_or("Missing backend kind")?)?,
        backend_address:
            args.next().ok_or("Missing backend address")?,
    })
}

fn parse_cmdline() -> Result<CmdlineArgs, String> {
    let mut args = std::env::args();
    args.next().unwrap();
    let res =
        match args.next().ok_or("Missing command")?.as_ref() {
            "pub" => CmdlineArgs::Pub(parse_service_args(&mut args)?),
            "sub" => CmdlineArgs::Sub(parse_sub_args(&mut args)?),
            "req" => CmdlineArgs::Req(parse_service_args(&mut args)?),
            "rep" => CmdlineArgs::Rep(parse_service_args(&mut args)?),
            "proxy" => CmdlineArgs::Proxy(parse_proxy_args(&mut args)?),
            cmd => return Err(format!("Unknown command {}", cmd))
        };
    match args.next() {
        None => (),
        Some(arg) => return Err(format!("Useless argument {}", arg))
    };
    Ok(res)
}

fn connect_or_bind(socket: &zmq::Socket, service: &Service) ->
    Result<(), String> {
    match service.connect_or_bind {
        ConnectOrBind::Connect => socket.connect(&service.address)
            .map_err(|msg| { format!("Unable to connect: {}", msg) }),
        ConnectOrBind::Bind => socket.bind(&service.address)
            .map_err(|msg| { format!("Unable to bind: {}", msg) }),
    }
}

fn send_line(socket: &zmq::Socket, line: &str, separator: &Option<String>) {
    match separator {
        &None => socket.send_str(line, 0).unwrap(),
        &Some(ref separator) => {
            let msgs = line
                .split(separator)
                .map(|s| s.bytes().collect())
                .collect::<Vec<Vec<u8>>>();
            let msgs = msgs
                .iter()
                .map(|x| x.as_slice())
                .collect::<Vec<&[u8]>>();
            socket.send_multipart(msgs.as_slice(), 0).unwrap();
        }
    }
}

fn receive_message(socket: &zmq::Socket, separator: &Option<String>) {
    let msgs = socket.recv_multipart(0).unwrap();
    if msgs.len() == 1 {
        println!("{}", String::from_utf8_lossy(&msgs[0]))
    }
    else {
        match separator {
            &None => {
                println!("BEGIN MULTIPART MESSAGE");
                for msg in msgs {
                    println!("{}", String::from_utf8_lossy(&msg))
                }
                println!("END MULTIPART MESSAGE");
            }
            &Some(ref separator) => {
                let mut iter = msgs.iter();
                match iter.next() {
                    None => (),
                    Some(msg) => {
                        print!("{}", String::from_utf8_lossy(&msg));
                        for msg in iter {
                            print!("{}", &separator);
                            print!("{}", String::from_utf8_lossy(&msg));
                        }
                        println!("");
                    }
                }
            }
        }
    }
}

fn run_pub(ctx: zmq::Context, args: Service) ->
    Result<(), String> {
    let socket = ctx.socket(zmq::PUB).unwrap();
    connect_or_bind(&socket, &args)?;
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        send_line(&socket, &line.unwrap(), &args.separator);
    }
    Ok(())
}

fn run_sub(ctx: zmq::Context, args: Sub) ->
    Result<(), String> {
    let socket = ctx.socket(zmq::SUB).unwrap();
    connect_or_bind(&socket, &args.service)?;
    socket.set_subscribe(args.filter.as_bytes()).unwrap();
    loop {
        receive_message(&socket, &args.service.separator);
    }
}

fn run_req(ctx: zmq::Context, args: Service) ->
    Result<(), String> {
    let socket = ctx.socket(zmq::REQ).unwrap();
    connect_or_bind(&socket, &args)?;
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        send_line(&socket, &line.unwrap(), &args.separator);
        receive_message(&socket, &args.separator);
    }
    Ok(())
}

fn run_rep(ctx: zmq::Context, args: Service) ->
    Result<(), String> {
    let socket = ctx.socket(zmq::REP).unwrap();
    connect_or_bind(&socket, &args)?;
    receive_message(&socket, &args.separator);
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        send_line(&socket, &line.unwrap(), &args.separator);
        receive_message(&socket, &args.separator);
    }
    Ok(())
}

fn run_proxy(ctx: zmq::Context, args: Proxy) ->
    Result<(), String> {
    let mut frontend = ctx.socket(args.frontend_kind).unwrap();
    frontend.bind(&args.frontend_address).unwrap();
    let mut backend = ctx.socket(args.backend_kind).unwrap();
    backend.bind(&args.backend_address).unwrap();
    zmq::proxy(&mut frontend, &mut backend).unwrap();
    Ok(())
}

fn main() {
    let args = parse_cmdline().unwrap_or_else(|msg| {
        println!("{}", msg);
        println!("Usage:
zmq pub|sub|req|rep [--multi-message <sep char>] <connect|bind> <address>
zmq sub [--multi-message <sep char>] <connect|bind> <address> <filter>
zmq proxy <frontend kind> <address> <backend kind> <address>");
        std::process::exit(1);
    });
    let ctx = zmq::Context::new();
    match args {
        CmdlineArgs::Pub(args) => run_pub(ctx, args),
        CmdlineArgs::Sub(args) => run_sub(ctx, args),
        CmdlineArgs::Req(args) => run_req(ctx, args),
        CmdlineArgs::Rep(args) => run_rep(ctx, args),
        CmdlineArgs::Proxy(args) => run_proxy(ctx, args),
    }.unwrap_or_else(|msg| {
        println!("{}", msg);
        std::process::exit(1);
    });
}
