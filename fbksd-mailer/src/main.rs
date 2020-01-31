//! fbksd-mailer binary.
//!
//! The mailer process is responsible for sending messages to users by e-mail.
//! The messages are read from the table `message_tasks` in the database.

use fbksd_core;
use fbksd_core::{db, system_config};

use log;
use log::LevelFilter;
use log4rs;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Config, Root};

use lettre::smtp::authentication::IntoCredentials;
use lettre::smtp::ConnectionReuseParameters;
use lettre::{SendableEmail, SmtpClient, Transport};
use lettre_email::Email;

use std::{thread, time};

fn main() {
    // config logger
    let stdout = ConsoleAppender::builder().build();
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))
        .unwrap();
    log4rs::init_config(config).unwrap();

    let config = system_config::SystemConfig::load();
    let credentials = (&config.mailer_email_user, &config.mailer_email_password);
    let polling_rate = time::Duration::from_secs(config.mailer_polling_rate as u64);
    let timeout = time::Duration::from_secs(config.mailer_timeout as u64);

    log::info!("Process started.");

    let mut client = SmtpClient::new_simple(&config.mailer_smtp_domain)
        .unwrap()
        .credentials(credentials.into_credentials())
        .timeout(Some(timeout))
        .connection_reuse(ConnectionReuseParameters::ReuseUnlimited)
        .transport();

    loop {
        let task = db::read_next_message();
        match task {
            Ok(Some(msg)) => {
                log::info!("Found new message to send to <{}>:", &msg.to_address);
                // keep trying to send message until success
                loop {
                    let email: SendableEmail = Email::builder()
                        .to(msg.to_address.clone())
                        .from(config.mailer_email_user.as_ref())
                        .subject(&msg.subject)
                        .text(&msg.text)
                        .build()
                        .unwrap()
                        .into();

                    log::info!("  Sending...");
                    let result = client.send(email.into());
                    if result.is_ok() {
                        log::info!("  Message sent.");
                        // pop the message only if the e-mail was send
                        match db::pop_next_message() {
                            Ok(()) => {
                                log::info!("  Message removed from queue.");
                                break;
                            }
                            Err(err) => {
                                log::error!("  Failed to remove message from queue: {}", err);
                                panic!();
                            }
                        }
                    } else {
                        log::error!("  Failed to send message. Error: {:?}\n  Retry.", result);
                    }
                }
            }
            _ => {}
        }
        thread::sleep(polling_rate);
    }
}
