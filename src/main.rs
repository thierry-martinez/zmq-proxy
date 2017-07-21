extern crate zmq;

struct CmdlineArgs {
    frontend_kind: zmq::SocketType,
    frontend_address: String,
    backend_kind: zmq::SocketType,
    backend_address: String,
}

fn socket_type_of_str(s: &str) -> Result<zmq::SocketType, String> {
    match s {
        "ROUTER" => Ok(zmq::ROUTER),
        "DEALER" => Ok(zmq::DEALER),
        "XSUB" => Ok(zmq::XSUB),
        "XPUB" => Ok(zmq::XPUB),
        _ => Err(format!("Unknown socket kind {}", s)),
    }
}

fn parse_cmdline() -> Result<CmdlineArgs, String> {
    Ok (CmdlineArgs {
        frontend_kind: socket_type_of_str(
            &std::env::args().nth(1).ok_or("Missing frontend kind")?)?,
        frontend_address:
            std::env::args().nth(2).ok_or("Missing frontend address")?,
        backend_kind: socket_type_of_str(
            &std::env::args().nth(3).ok_or("Missing backend kind")?)?,
        backend_address:
            std::env::args().nth(4).ok_or("Missing backend address")?,
    })
}

fn main() {
    let args = parse_cmdline().unwrap_or_else(|msg| {
        println!("{}", msg);
        println!("Usage: zmq-proxy \
            <frontend kind> <address> <backend kind> <address>");
        std::process::exit(1);
    });
    let ctx = zmq::Context::new();
    let mut frontend = ctx.socket(args.frontend_kind).unwrap();
    frontend.bind(&args.frontend_address).unwrap();
    let mut backend = ctx.socket(args.backend_kind).unwrap();
    backend.bind(&args.backend_address).unwrap();
    zmq::proxy(&mut frontend, &mut backend).unwrap();
}
