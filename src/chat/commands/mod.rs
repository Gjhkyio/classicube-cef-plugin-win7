mod helpers;
mod options_commands;
mod screen_commands;
mod self_commands;
mod static_commands;

use super::{Chat, PlayerSnapshot};
use crate::error::*;
use clap::{App, AppSettings, ArgMatches};
use classicube_helpers::OptionWithInner;
use std::cell::RefCell;

thread_local!(
    static COMMAND_APP: RefCell<Option<App<'static, 'static>>> = Default::default();
);

pub fn initialize() {
    let app = App::new("cef")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .global_setting(AppSettings::ColoredHelp)
        .global_setting(AppSettings::DisableVersion)
        .global_setting(AppSettings::DisableHelpFlags);

    #[cfg(not(test))]
    let app = app.global_setting(AppSettings::ColorAlways);

    #[cfg(test)]
    let app = app.global_setting(AppSettings::ColorNever);

    let app = static_commands::add_commands(app);
    let app = self_commands::add_commands(app);
    let app = options_commands::add_commands(app);
    let app = screen_commands::add_commands(app);

    COMMAND_APP.with(|cell| {
        let cell = &mut *cell.borrow_mut();
        *cell = Some(app);
    });
}

pub fn get_matches(args: &[String]) -> Result<ArgMatches<'static>> {
    Ok(COMMAND_APP
        .with_inner_mut(|app| app.get_matches_from_safe_borrow(args))
        .unwrap()?)
}

pub async fn run(player: PlayerSnapshot, mut args: Vec<String>, is_self: bool) -> Result<()> {
    args.insert(0, "cef".to_string());

    log::debug!("command {:?}", args);

    match get_matches(&args) {
        Ok(matches) => {
            if static_commands::handle_command(&player, &matches).await?
                || (is_self && self_commands::handle_command(&player, &matches).await?)
                || (is_self && options_commands::handle_command(&player, &matches).await?)
                || screen_commands::handle_command(&player, &matches).await?
            {
                Ok(())
            } else {
                bail!("command not handled? {:?}", args);
            }
        }

        Err(e) => {
            log::warn!("{:#?}", e);
            chat_print_lines(format!("{}", e));
            Ok(())
        }
    }
}

/// needs to keep same color code from last line
fn chat_print_lines(s: String) {
    let s = s.trim();

    let lines: Vec<String> = s.split('\n').map(String::from).collect();

    let mut last_color = None;
    for line in lines {
        let message = if let Some(last_color) = last_color.filter(|c| *c != 'f') {
            format!("&{}{}", last_color, line)
        } else {
            line
        };

        Chat::print(&message);

        last_color = get_last_color(&message);
    }
}

fn get_last_color(text: &str) -> Option<char> {
    let mut last_color = None;
    let mut found_ampersand = false;

    for c in text.chars() {
        if c == '&' {
            found_ampersand = true;
        } else if found_ampersand {
            found_ampersand = false;
            last_color = Some(c);
        }
    }

    last_color
}

#[tokio::test]
async fn test_commands() {
    crate::logger::initialize(true, true);
    initialize();

    unsafe {
        run(std::mem::zeroed(), vec!["create".into(), "ag".into()], true)
            .await
            .unwrap();
    }
}