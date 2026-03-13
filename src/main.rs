use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
use colored::Colorize;
use dialoguer::{Input, Select, theme::ColorfulTheme};

mod browser;
mod config;
mod linkedin;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = config::Config::load();
    let (mut b, page, handle) = browser::create_browser().await?;

    page.goto("https://linkedin.com").await?;

    page.execute(
        SetDeviceMetricsOverrideParams::builder()
            .width(1220u32)
            .height(760u32)
            .device_scale_factor(1.0)
            .mobile(false)
            .build()?,
    )
    .await?;

    println!("\n{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".blue());
    println!(
        "  {} {}",
        "🔗 Linky".bold().white(),
        "- LinkedIn Automation".dimmed()
    );
    println!("{}\n", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".blue());

    loop {
        let options = vec!["Connect", "Comment", "Settings", "Quit"];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What do you want to do?")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => {
                let amount: i8 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("How many connections?")
                    .default(cfg.default_connect_amount)
                    .interact()?;
                println!(
                    "\n{} Connecting to {} people...\n",
                    "→".blue(),
                    amount.to_string().bold()
                );
                linkedin::connections::connect(&page, amount).await?;
                println!("{} Done!\n", "✓".green().bold());
            }
            1 => {
                let amount: i8 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("How many comments?")
                    .default(cfg.default_comment_amount)
                    .interact()?;
                println!(
                    "\n{} Commenting on {} posts...\n",
                    "→".blue(),
                    amount.to_string().bold()
                );

                if cfg.gemini_api_key.is_empty() {
                    println!(
                        "{} No API key set. Go to Settings → Set API Key first.\n",
                        "✗".red().bold()
                    );
                    continue;
                }

                linkedin::feed::comment_posts(
                    &page,
                    amount,
                    cfg.rating_threshold,
                    cfg.gemini_api_key.clone(),
                    cfg.rating_sleep_ms,
                    cfg.comment_sleep_ms,
                )
                .await?;
                println!("{} Done!\n", "✓".green().bold());
            }
            2 => {
                let settings_options = vec!["Set API Key", "Set Default Amounts", "Back"];

                let settings_selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Settings")
                    .items(&settings_options)
                    .default(0)
                    .interact()?;

                match settings_selection {
                    0 => {
                        let key: String = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("Enter Gemini API key")
                            .interact()?;
                        cfg.gemini_api_key = key;
                        cfg.save();
                        println!("{} API key saved\n", "✓".green().bold());
                    }
                    1 => {
                        cfg.default_connect_amount = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("Default connect amount")
                            .default(cfg.default_connect_amount)
                            .interact()?;
                        cfg.default_comment_amount = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("Default comment amount")
                            .default(cfg.default_comment_amount)
                            .interact()?;
                        cfg.rating_threshold = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("Rating threshold (1-10)")
                            .default(cfg.rating_threshold)
                            .interact()?;
                        cfg.rating_sleep_ms = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("Sleep between rating posts (ms)")
                            .default(cfg.rating_sleep_ms)
                            .interact()?;
                        cfg.comment_sleep_ms = Input::with_theme(&ColorfulTheme::default())
                            .with_prompt("Sleep before posting comment (ms)")
                            .default(cfg.comment_sleep_ms)
                            .interact()?;
                        cfg.save();
                        println!("{} Settings saved\n", "✓".green().bold());
                    }
                    _ => {}
                }
            }
            3 => break,
            _ => {}
        }
    }

    b.close().await?;
    handle.await?;
    Ok(())
}
