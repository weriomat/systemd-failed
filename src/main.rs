use anyhow::Result;
// use clap::Parser;
use log::{error, info, LevelFilter};
use rustix;
use std::{
    borrow::Cow,
    fmt::Display,
    process::{Command, Stdio},
    str::FromStr,
};
use systemd_journal_logger::JournalLog;

// /// Simple program to querry failed systemd units and notify given email
// #[derive(Parser, Debug)]
// #[command(version, about, long_about = None)]
// struct Args {
//     /// Email to send a Mail to
//     #[arg(short, long, default_value_t = String::from("engel@weriomat.com"))]
//     email: String,
// }

#[derive(Debug)]
struct FailedUnits {
    number: usize,
    messages: Vec<String>,
    names: Vec<String>,
}

impl Display for FailedUnits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // NOTE: we assume that each entry is 20 chars long + 18 for prelude
        let mut tmp = String::with_capacity((self.number + 2) * 20 + 18);
        if self.number == 0 {
            tmp += "No failed units";
        } else {
            tmp += "Failed Units: [ ";
            if self.number == 1 {
                for it in &self.names {
                    tmp.push_str(it);
                }
            } else {
                for it in &self.names {
                    tmp.push_str(it);
                    tmp.push_str(", ");
                }
            }
            tmp += " ]";
        }
        write!(f, "{}", tmp)
    }
}

impl FailedUnits {
    pub fn new() -> Self {
        FailedUnits {
            number: 0,
            messages: Vec::new(),
            names: Vec::new(),
        }
    }
    pub fn add_failed(&mut self, s: String) {
        self.number += 1;
        // NOTE: cow is dumb here
        let cow = Cow::from(s);
        self.messages.push(cow.clone().into_owned());

        match String::from_str(cow.as_ref().trim_start()) {
            Ok(mut st) => {
                let beta_offset = st.find('●').unwrap_or(st.len());
                let _ = st.drain(..beta_offset).collect::<String>();
                match st.split_once(' ') {
                    Some((_, ac)) => match ac.split_once(' ') {
                        Some((ab, _)) => {
                            self.names.push(ab.into());
                        }
                        None => error!("Systemd failed: Cannot split String: {st}"),
                    },
                    None => error!("Systemd failed: Cannot split String: {st}"),
                }
            }
            Err(err) => error!("Systemd Failed: Cannot Trim Start: {err}"),
        }
    }
}

/// Run the check
fn run_check(mail: String) -> Result<FailedUnits> {
    // convert to string
    let mut failed_units = String::from_utf8(
        Command::new("systemctl")
            .arg("--failed")
            .output()?
            .stdout
            .as_slice()
            .to_vec(),
    )?;

    // discard header
    let beta_offset = failed_units.find('●').unwrap_or(failed_units.len());
    let pre = failed_units.drain(..beta_offset).collect::<String>();

    let mut fu = FailedUnits::new();

    // get failed units
    let new_ln = failed_units.find('\n').unwrap_or(failed_units.len());
    let f = failed_units.drain(..new_ln).collect::<String>();
    if !f.is_empty() {
        // TODO: parse more
        // TODO: systemctl status --full

        // prepare string to send
        let hostname = String::from_utf8(rustix::system::uname().nodename().to_bytes().to_vec())?;
        let mut string_to_send = String::from(format!("To: {}\r\nFrom: systemd <root@{}>\r\nContent-Transfer-Encoding: 8bit\r\nContent-Type: text/plain; charset=UTF-8\r\nSubject: Failed Systemd-Unit\r\n\r\n", mail,hostname));
        string_to_send.push_str(&pre);
        string_to_send.push_str(&f);
        // string_to_send.push_str("\"");
        println!("a: {string_to_send}");

        // Add failed unit
        fu.add_failed(f);

        // send mail
        // echo -e "Content-Type: text/plain\r\nSubject: Test\r\n\r\nHello woiruiwoeurweoiru Worldtesti" | sendmail -vv engel@weriomat.com
        let echo_child = Command::new("echo")
            .arg("-e")
            .arg(string_to_send)
            .stdout(Stdio::piped())
            .spawn()?;

        let mail = Command::new("sendmail")
            .arg(mail)
            .stdin(Stdio::from(
                echo_child.stdout.expect("Failed to open stdout"),
            ))
            .stdout(Stdio::piped())
            .spawn()?;

        // TODO: sendmail
        // TODO: parse sendmail

        // let sed_child = Command::new("sed")
        //     .arg("weiur")
        //     .stdin(Stdio::from(
        //         echo_child.stdout.expect("Failed to open echo stdout"),
        //     ))
        //     .stdout(Stdio::piped())
        //     .spawn()?;

        let output = mail.wait_with_output()?;
        info!("Systemd-failed: {output:?}");
    }
    Ok(fu)
}

fn main() {
    JournalLog::new().unwrap().install().unwrap();
    log::set_max_level(LevelFilter::Info);

    // parse the args
    // let args = Args::parse();

    info!("Systemd failed started");

    if !cfg!(unix) {
        println!("Error: Platfrom is non UNIX -> cant run systemd");
        return;
    }

    // match run_check(args.email) {
    match run_check("engel@weriomat.com".into()) {
        Ok(val) => {
            println!("Res: {val:?}");
            if val.number == 0 {
                info!("Systemd failed: {val}");
            } else {
                info!("Systemd failed: {val} unit[s] failed");
            }
        }
        Err(err) => {
            error!("Command failed: {err}");
        }
    }
}
