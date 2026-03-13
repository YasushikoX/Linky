use std::time::Duration;

use chromiumoxide::Page;
use tokio::time::sleep;

pub async fn connect(
    page: &Page,
    connection_ammount: i8,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut count: i8 = 0;

    page.goto("https://www.linkedin.com/mynetwork").await?;

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    page.evaluate("document.querySelectorAll('iframe')[1].contentDocument.querySelector('#msg-overlay')?.remove()").await?;

    // Accept Invites Logic
    let invite_buttons = page.find_elements("button[aria-label*='Accept']").await?;

    println!(
        "Found {} people to accept invites from.",
        invite_buttons.len()
    );

    for btn in invite_buttons {
        btn.click().await?;
        sleep(Duration::from_secs_f32(0.5)).await;
    }

    // New Connection Logic
    for _ in 0..(connection_ammount / 20) {
        page.evaluate("document.querySelector('main#workspace').scrollTop += 2000")
            .await?;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    let buttons = page.find_elements("button[aria-label*='connect']").await?;

    println!("Found {} people to connect with.", buttons.len());

    for btn in buttons {
        count += 1;
        btn.click().await?;
        sleep(Duration::from_secs_f32(0.5)).await;
        if count >= connection_ammount {
            break;
        }
    }

    println!("Finished Connecting");

    Ok(())
}
