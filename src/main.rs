use std::{
    io::{stderr, stdout},
    net::Ipv4Addr,
    str::from_utf8,
    time::Duration,
};

use clap::Parser;
use crossterm::{
    style::{Color, Print, ResetColor, SetForegroundColor},
    ExecutableCommand,
};
use tokio::{
    io::AsyncReadExt,
    net::{TcpStream, ToSocketAddrs},
    select,
    time::sleep,
};

const PORT: u16 = 1741;

#[derive(Parser)]
struct Args {
    team: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    loop {
        let _ = run(args.team).await;

        stderr()
            .execute(SetForegroundColor(Color::Red))
            .unwrap()
            .execute(Print("Disconnected\n"))
            .unwrap()
            .execute(ResetColor)
            .unwrap();
    }
}

pub fn get_addr(team_number: u16) -> impl ToSocketAddrs {
    (
        Ipv4Addr::new(10, (team_number / 100) as u8, (team_number % 100) as u8, 2),
        PORT,
    )
}

async fn run(team_number: u16) -> anyhow::Result<()> {
    let mut socket = loop {
        select! {
           stream = TcpStream::connect(get_addr(team_number)) => {
               if let Ok(stream) = stream {
                   break stream;
               }
           },
           _ = sleep(Duration::from_millis(300)) => {}
        }
    };

    stderr()
        .execute(SetForegroundColor(Color::Green))?
        .execute(Print("Connected\n"))?
        .execute(ResetColor)?;

    loop {
        handle_socket(&mut socket).await?;
    }
}

pub async fn handle_socket(socket: &mut TcpStream) -> anyhow::Result<()> {
    let len = socket.read_u16().await? as usize;

    let mut buffer = vec![0; len];

    socket.read_exact(&mut buffer).await?;

    let mut buffer = buffer.as_slice();

    let tag = parse_u8(&mut buffer);

    let timestamp = parse_f32(&mut buffer);
    let _seq = parse_i16(&mut buffer);

    stdout()
        .execute(SetForegroundColor(Color::DarkGrey))?
        .execute(Print(format!("{:.4} - ", timestamp)))?
        .execute(ResetColor)?;

    if tag == 11 {
        let num_occur = parse_i16(&mut buffer);
        let error_code = parse_i32(&mut buffer);
        let flags = parse_u8(&mut buffer);
        let details = parse_string(&mut buffer);
        let location = parse_string(&mut buffer);
        let callstack = parse_string(&mut buffer);

        let is_error = flags & 1 != 0;

        if is_error {
            stdout()
                .execute(SetForegroundColor(Color::Red))?
                .execute(Print(format!(
                    "ERROR {} {}:\n",
                    error_code,
                    if num_occur > 1 {
                        format!("({}x)", num_occur)
                    } else {
                        format!("")
                    }
                )))?;
        } else {
            stdout()
                .execute(SetForegroundColor(Color::DarkYellow))?
                .execute(Print(format!(
                    "WARNING {} {}:\n",
                    error_code,
                    if num_occur > 1 {
                        format!("({}x)", num_occur)
                    } else {
                        format!("")
                    }
                )))?;
        }

        stdout()
            .execute(ResetColor)?
            .execute(Print(format!("{} at {}\n{}", details, location, callstack)))?;
    } else if tag == 12 {
        let message = from_utf8(buffer).unwrap();

        let mut message = message.to_owned();

        message = message.replace(
            "TRACE",
            &format!("{}TRACE{}", SetForegroundColor(Color::Cyan), ResetColor),
        );
        message = message.replace(
            "INFO",
            &format!("{}INFO{}", SetForegroundColor(Color::Green), ResetColor),
        );
        message = message.replace(
            "DEBUG",
            &format!("{}DEBUG{}", SetForegroundColor(Color::Blue), ResetColor),
        );
        message = message.replace(
            "WARN",
            &format!(
                "{}WARN{}",
                SetForegroundColor(Color::DarkYellow),
                ResetColor
            ),
        );
        message = message.replace(
            "ERROR",
            &format!("{}ERROR{}", SetForegroundColor(Color::DarkRed), ResetColor),
        );
        message += "\n";

        stdout().execute(Print(message))?;
    }

    Ok(())
}

fn parse_u8(buf: &mut &[u8]) -> u8 {
    let data = buf[0];

    *buf = &buf[1..];

    data
}

fn parse_f32(buf: &mut &[u8]) -> f32 {
    let data = f32::from_be_bytes(buf[0..4].try_into().unwrap());

    *buf = &buf[4..];

    data
}

fn parse_i16(buf: &mut &[u8]) -> i16 {
    let data = i16::from_be_bytes(buf[0..2].try_into().unwrap());

    *buf = &buf[2..];

    data
}

fn parse_u16(buf: &mut &[u8]) -> u16 {
    let data = u16::from_be_bytes(buf[0..2].try_into().unwrap());

    *buf = &buf[2..];

    data
}

fn parse_i32(buf: &mut &[u8]) -> i32 {
    let data = i32::from_be_bytes(buf[0..4].try_into().unwrap());

    *buf = &buf[4..];

    data
}

fn parse_string<'a>(buf: &mut &'a [u8]) -> &'a str {
    let len = parse_u16(buf) as usize;

    dbg!(len);

    let string = &buf[..len];

    *buf = &buf[len..];

    from_utf8(string).unwrap()
}
