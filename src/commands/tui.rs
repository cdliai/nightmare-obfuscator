use anyhow::Result;
use colored::Colorize;
use std::io::{self, IsTerminal, Write};
use std::thread;
use std::time::Duration;

pub struct TuiArgs {
    pub preview: bool,
    pub account_name: Option<String>,
    pub password: Option<String>,
}

pub async fn run(args: TuiArgs) -> Result<()> {
    if args.preview {
        print!("{}", entry_screen());
        return Ok(());
    }

    if args.account_name.is_some() || args.password.is_some() {
        return reject_account_login();
    }

    if io::stdout().is_terminal() {
        run_interactive()
    } else {
        print!("{}", entry_screen());
        Ok(())
    }
}

fn run_interactive() -> Result<()> {
    play_logo_animation()?;
    print!("{}", entry_screen());
    print!("Select login mode [public/account/quit]: ");
    io::stdout().flush()?;

    let mut mode = String::new();
    io::stdin().read_line(&mut mode)?;
    match mode.trim().to_ascii_lowercase().as_str() {
        "" | "public" | "p" | "1" => {
            println!("{}", shell_screen());
            Ok(())
        }
        "account" | "a" | "2" => {
            let _account_name = prompt("Account name")?;
            let _password = prompt("Password")?;
            reject_account_login()
        }
        "quit" | "q" => Ok(()),
        other => anyhow::bail!("unknown login mode: {other}"),
    }
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}: ");
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim().to_string())
}

fn reject_account_login() -> Result<()> {
    anyhow::bail!("Account login is not connected yet. Use Public Login for the local shell.")
}

fn play_logo_animation() -> Result<()> {
    for frame in logo_animation_frames() {
        print!("\x1b[2J\x1b[H{frame}");
        io::stdout().flush()?;
        thread::sleep(Duration::from_millis(90));
    }
    Ok(())
}

pub fn entry_screen() -> String {
    format!(
        "{logo}\n{rule}\n{title}\n{subtitle}\n\n{public}\n{account}\n\n{hint}\n{rule}\n",
        logo = cdli_logo_ascii(),
        rule = "+------------------------------------------------------------+".dimmed(),
        title = "CDLI.ai Nightmare Obfuscator".red().bold(),
        subtitle = "cdli.ai | controlled source sharing for partner drops".dimmed(),
        public = "[1] Public Login   local project obfuscation shell".green(),
        account = "[2] Account Login  CDLI account access (not connected yet)".yellow(),
        hint = "Run: public -> configure source/output/checks; account -> rejected until backend lands"
            .dimmed(),
    )
}

fn shell_screen() -> &'static str {
    r#"
+------------------------------------------------------------+
| Nightmare Shell                                            |
|                                                            |
|  [I] Init contract       [R] Run obfuscation               |
|  [V] Verify output       [G] Gate / plan repo              |
|                                                            |
| This shell is backed by nightmare.toml and nightmare run.   |
+------------------------------------------------------------+
"#
}

fn logo_animation_frames() -> [&'static str; 3] {
    [
        "CDLI.ai\n\nloading identity surface...\n",
        "CDLI.ai\n\nrendering source vault...\n",
        cdli_logo_ascii(),
    ]
}

pub fn cdli_logo_ascii() -> &'static str {
    // ASCII reduction of the CDLI logo mark. Keep this stable; it is the
    // terminal companion to the canonical CDLI brand asset.
    r#"@@@@@@@@@@@@@@@@@@@@#=::+%@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@#=:.:=*+-..-+#@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@*=. .=#@@@#@%+=. :+%@@@@@@@@@@@@@
@@@@@@@@@#+#%*=: -+#@@+-.:+@@%*. :@@#@@@@@@@@@
@@@@@%*=. . .=#@#=: .=#@+ :@@@@= :@# :=#@@@@@@
%#+-::-+#@@%*=: :=*%#+-.. :@@@@=  @# .+-.:+#@@
. .=#@@#*%@@@@@#+:  -*@%+-.@@@@=  @# :@@%+- .=
  #@%:.-=:.-+#@@@@%#+- .-+#@@@@= :@# -@@-+@%
  #@%#@@*= .-+%%#=.-*@%*=. :=#@= :@# .@@  @#
  *#+-.:=*%#+-:.-+#@@@@@@@%*==@= :@#  ::=*@#
  . :+#@#+: .=*%@@@@@@@@@@@@@@@= :@#-+#@@@@#
-*#%*-:.:=*%@@- +@@@@@@@@@@@@@@= :@@@@@%+-..-+
#+- .:+%@@@@@@  +@@@@@@@@@@@@@@-:+@@#+: .=*%%+
  -%@@@@%*=-%@. +@@@@@@@@@@@@@@%*=- :=*%#+-:
  #@@%+-    #@- +@#%@@@@@@@@%+-. -+#@#=: :=+
  #@@: =%%  #@: =@*:.-+#@@#::-+%#+=:.-+#@%@#
  #@@= #@#  #@: +@@@#=. :=#@@@@@#+: :=*+:-@#
-.:=*%@@@@  #@: +@@@#+#%*=: :=#@@@@@#+=+#%#=
@%+-..-*%%  #@. +@@@+  .=#@#=. .=*@@@@@#=: :=#
@@@@@%+-..  #@  +@@@+ =#=-.-=##*+:.:=:.-+#@@@@
@@@@@@@@@*=.#@  +@@@+ :+%@%+- .=#@#+-*%@@@@@@@
@@@@@@@@@@@@@@-..-+#@@*+*@@#+- :=*@@@@@@@@@@@@
@@@@@@@@@@@@@@@%#=. :+%%#=. -+#@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@*=:.-+#@@@@@@@@@@@@@@@@@@@"#
}
