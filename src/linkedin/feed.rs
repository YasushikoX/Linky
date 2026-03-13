use std::{collections::HashSet, time::Duration};

use chromiumoxide::Page;
use google_ai_rs::Client;

pub async fn comment_posts(
    page: &Page,
    comments_amount: i8,
    rating_threshold: i32,
    key: String,
    rating_sleep_ms: u64,
    comment_sleep_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut count: i8 = 0;
    let mut commented_ids: HashSet<String> = HashSet::new();

    page.goto("https://www.linkedin.com/feed/").await?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Removes messaging window
    page.evaluate("document.querySelectorAll('iframe')[1].contentDocument.querySelector('#msg-overlay')?.remove()").await?;

    while count < comments_amount {
        // index, unique id, text, rating
        let mut posts_data: Vec<(usize, String, String, i32)> = Vec::new();

        // Get posts text for evaluation
        let posts = page.find_elements("[role='listitem']").await?;

        for (i, _post) in posts.iter().enumerate() {
            let id: String = page
                .evaluate(format!(
                    r#"(() => {{
                        const el = document.querySelectorAll("[role='listitem']")[{}];
                        return el.getAttribute('data-urn')
                            || el.getAttribute('data-id')
                            || el.getAttribute('id')
                            || el.innerText.slice(0, 100);
                    }})()"#,
                    i
                ))
                .await?
                .into_value()?;

            if commented_ids.contains(&id) {
                continue;
            }

            let text: String = page
                .evaluate(format!(
                    "document.querySelectorAll(\"[role='listitem']\")[{}]
                    .querySelector(\"[data-testid='expandable-text-box']\")?.innerText ?? ''",
                    i
                ))
                .await?
                .into_value()?;

            posts_data.push((i, id, text, 0));
        }

        // Evaluate posts commentability
        for (_i, _id, text, rating) in posts_data.iter_mut() {
            let response = request(
                key.clone(),
                format!(
                    "Rate this LinkedIn post's commentability from 1-10 based on how easy it is to leave a comment. \
                    High scores (8-10): personal experiences, lessons learned, opinions, stories, personal achievements. \
                    Low scores (1-3): job search announcements, advertisement, generic motivational quotes. \
                    Reply with ONLY a single number, nothing else. \
                    \n\nPost:\n{}",
                    text
                ),
                "gemini-3.1-flash-lite-preview".to_string(),
            )
            .await?;

            *rating = response.trim().parse().unwrap_or(0);

            tokio::time::sleep(Duration::from_millis(rating_sleep_ms)).await;
        }

        // Filter by threshold
        let filtered: Vec<(usize, String, String, i32)> = posts_data
            .into_iter()
            .filter(|(_i, _id, _text, score)| *score >= rating_threshold)
            .collect();

        println!("Found {} to comment on", filtered.len());

        if filtered.is_empty() {
            page.evaluate("document.querySelector('main#workspace').scrollTop += 800")
                .await?;
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        // Comment on the posts
        for (i, id, text, _rating) in filtered {
            if count >= comments_amount {
                break;
            }

            let response = request(
                key.clone(),
                format!(
                    "Write a short genuine comment for this LinkedIn post. \
                    Be positive, appreciative and nice. \
                    Be conversational and human. \
                    Be somewhat casual. \
                    Use simple, conversational words. \
                    Max 2 sentences. No hashtags. No emojis. \
                    Respond with the comment only, nothing else.\
                    \n\nPost:\n{}",
                    text
                ),
                "gemini-3-flash-preview".to_string(),
            )
            .await?;

            let comment = response.trim().replace('\'', "\\'").replace('\n', " ");

            // Click comment button
            let expanded: bool = page
                .evaluate(format!(
                    r#"(() => {{
                        const btn = Array.from(document.querySelectorAll("[role='listitem']")[{}].querySelectorAll('button'))
                            .find(b => b.innerText.includes('Comment'));
                        if (btn) {{ btn.click(); return true; }}
                        return false;
                    }})()"#,
                    i
                ))
                .await?
                .into_value()?;

            if !expanded {
                eprintln!("⚠️ could not find comment button for post {} — skipping", i);
                continue;
            }

            tokio::time::sleep(Duration::from_secs(1)).await;

            // Type comment
            page.evaluate(format!(
                r#"(() => {{
                    const item = document.querySelectorAll('[role=\'listitem\']')[{}];
                    const editor = item.querySelector("[aria-label='Text editor for creating comment']");
                    if (!editor) return false;
                    editor.focus();
                    document.execCommand('insertText', false, '{}');
                    return true;
                }})()"#,
                i, comment
            ))
            .await?;

            tokio::time::sleep(Duration::from_millis(comment_sleep_ms)).await;

            // Submit comment
            let posted: bool = page
                .evaluate(format!(
                    r#"(() => {{
                        const item = document.querySelectorAll('[role=\'listitem\']')[{}];
                        const btn = item.querySelector("button[componentkey*='commentButtonSection']");
                        if (btn) {{ btn.click(); return true; }}
                        return false;
                    }})()"#,
                    i
                ))
                .await?
                .into_value()?;

            if !posted {
                eprintln!("⚠️ could not post comment for post {} — skipping", i);
                continue;
            }

            commented_ids.insert(id);
            count += 1;

            println!("Completed {}/{} comments", count, comments_amount);

            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        // Scroll to load more posts
        for _ in 0..3 {
            page.evaluate("document.querySelector('main#workspace').scrollTop += 2000")
                .await?;
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    println!("Completed {} comments", count);

    Ok(())
}

// gemini-3-flash-preview
// gemini-3.1-flash-lite-preview
pub async fn request(
    key: String,
    prompt: String,
    model: String,
) -> Result<String, Box<dyn std::error::Error>> {
    let max_retries = 5;
    let mut wait_secs = 10;

    for attempt in 1..=max_retries {
        let client = Client::new(key.clone()).await?;
        let m = client.generative_model(&model);
        let mut chat = m.start_chat();

        match chat.send_message(prompt.clone()).await {
            Ok(response) => return Ok(response.to_string()),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("429") || msg.contains("rate") || msg.contains("quota") {
                    println!(
                        "⚠️ Rate limit hit, waiting {}s before retry {}/{}...",
                        wait_secs, attempt, max_retries
                    );
                    tokio::time::sleep(Duration::from_secs(wait_secs)).await;
                    wait_secs *= 2; // exponential backoff
                } else {
                    return Err(Box::new(e));
                }
            }
        }
    }

    Err("Rate limit: max retries exceeded".into())
}
