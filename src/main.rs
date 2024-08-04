use anyhow::Result;
use clap::Parser;
use lettre::{message::header::ContentType, Message, SendmailTransport, Transport};
use log::{error, info, LevelFilter};
use std::{fmt::Display, process::Command};
use systemd_journal_logger::JournalLog;

/// Simple program to querry failed systemd units and notify given email
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Email to send a Mail to
    #[arg(short, long, default_value_t = String::from("engel@weriomat.com"))]
    email: String,
}

#[derive(Debug)]
struct FailedUnits {
    number: usize,
    systemctl_full: Vec<String>,
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
            names: Vec::new(),
            systemctl_full: Vec::new(),
        }
    }
    pub fn add_failed(&mut self, s: String) {
        self.number += 1;

        // get first whitespace and drop everything else
        let mut iter = s.as_str().split_whitespace().skip(1);
        if let Some(unit) = iter.next() {
            match Command::new("systemctl")
                .args(vec!["status", "--full", unit])
                .output()
            {
                Ok(o) => match String::from_utf8(o.stdout.as_slice().to_vec()) {
                    Ok(fuo) => {
                        self.systemctl_full.push(fuo);
                    }
                    Err(err) => {
                        error!("Systemd failed: Cannot convert the output of `systemctl status --full {}` -> {err}", unit)
                    }
                },
                Err(err) => {
                    error!("Systemd failed: Cannot get the result of `systemctl status --full {}` -> {err}", unit);
                }
            }
            self.names.push(String::from(unit));
        }
    }

    pub fn mail(&self, args: Args) -> Result<()> {
        // construct the body of the email
        let mut body = String::from("Failed units:\r\n");
        let mut full = String::from("Systemctl status output of failed units:\r\n");

        for i in self.names.iter().enumerate() {
            body += "\r\n";
            body += i.1;
            full += "\r\n\r\n\r\n";
            full += &self.systemctl_full[i.0];
        }

        body = format!("{}\r\n\r\n{}", body, full);

        // send mail
        let hostname = String::from_utf8(rustix::system::uname().nodename().to_bytes().to_vec())?;

        // using lettre
        let email = Message::builder()
            .from((format!("systemd {} <mail@weriomat.com>", hostname)).parse()?)
            .to((format!("admin <{}>", args.email)).parse()?)
            .subject("Failed Systemd-Units")
            .header(ContentType::TEXT_PLAIN)
            .body(body)?;

        SendmailTransport::new().send(&email)?;
        Ok(())
    }
}

/// Run the check
fn run_check(args: Args) -> Result<FailedUnits> {
    // convert to string
    let failed_units = String::from_utf8(
        Command::new("systemctl")
            .arg("--failed")
            .output()?
            .stdout
            .as_slice()
            .to_vec(),
    )?;

    let mut fu = FailedUnits::new();

    // TODO: use memchr
    // parse each line
    for line in failed_units.lines() {
        // we trim the start so we just take the lines starting with '●'
        if line.trim_start().starts_with('●') {
            fu.add_failed(line.into());
        }
    }

    if fu.number != 0 {
        // TODO: cache units and send mail when resolved
        // in case we have a failed unit -> send email
        fu.mail(args)?;
    }

    Ok(fu)
}

fn main() {
    JournalLog::new().unwrap().install().unwrap();
    log::set_max_level(LevelFilter::Info);

    // parse the args
    let args = Args::parse();

    info!("Systemd failed started");

    if !cfg!(unix) {
        println!("Error: Platfrom is non UNIX -> cant run systemd");
        return;
    }

    match run_check(args) {
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
