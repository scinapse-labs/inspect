use std::path::PathBuf;

use clap::Args;
use colored::Colorize;
use sem_core::git::types::DiffScope;

use crate::OutputFormat;
use inspect_core::analyze::analyze;
use inspect_core::llm::{AnthropicClient, OpenAIClient, LlmProvider, EntityLlmReview, LlmVerdict};
use inspect_core::types::RiskLevel;

#[derive(Args)]
pub struct ReviewArgs {
    /// Commit ref, range, or PR number (with --remote)
    pub target: String,

    /// Output format
    #[arg(long, value_enum, default_value = "terminal")]
    pub format: OutputFormat,

    /// Minimum risk level to review (default: high)
    #[arg(long, default_value = "high")]
    pub min_risk: String,

    /// Model to use (e.g. claude-sonnet-4-5-20250929, gpt-4o, llama3)
    #[arg(long, default_value = "claude-sonnet-4-5-20250929")]
    pub model: String,

    /// Max entities to send for LLM review
    #[arg(long, default_value = "10")]
    pub max_entities: usize,

    /// Repository path
    #[arg(short = 'C', long, default_value = ".")]
    pub repo: PathBuf,

    /// LLM provider: anthropic, openai, ollama. Inferred from --api-base if omitted.
    #[arg(long)]
    pub provider: Option<String>,

    /// Custom API base URL (e.g. http://localhost:8000/v1). Implies openai provider.
    #[arg(long)]
    pub api_base: Option<String>,

    /// API key (overrides env var)
    #[arg(long)]
    pub api_key: Option<String>,

    /// Remote repo (e.g. owner/repo). Target becomes PR number.
    #[arg(long)]
    pub remote: Option<String>,

    /// Review strategy (for remote reviews)
    #[arg(long)]
    pub strategy: Option<String>,

    /// Timeout in seconds for remote review polling (default: 120)
    #[arg(long, default_value = "120")]
    pub timeout: u64,
}

fn build_provider(args: &ReviewArgs) -> Result<Box<dyn LlmProvider>, String> {
    // Infer provider: explicit flag > api-base implies openai > default anthropic
    let provider = args
        .provider
        .as_deref()
        .unwrap_or_else(|| if args.api_base.is_some() { "openai" } else { "anthropic" });

    match provider {
        "anthropic" => {
            let client = AnthropicClient::new(&args.model, args.api_key.as_deref())?;
            Ok(Box::new(client))
        }
        "openai" => {
            let client = OpenAIClient::new(
                &args.model,
                args.api_base.as_deref(),
                args.api_key.as_deref(),
            )?;
            Ok(Box::new(client))
        }
        "ollama" => {
            let base = args
                .api_base
                .as_deref()
                .unwrap_or("http://localhost:11434/v1");
            let client = OpenAIClient::new(&args.model, Some(base), None)?;
            Ok(Box::new(client))
        }
        other => Err(format!(
            "Unknown provider '{}'. Use: anthropic, openai, ollama",
            other
        )),
    }
}

pub async fn run(args: ReviewArgs) {
    if args.remote.is_some() {
        return run_remote(args).await;
    }

    let scope = parse_scope(&args.target);
    let repo = args.repo.canonicalize().unwrap_or(args.repo.clone());

    let mut result = match analyze(&repo, scope) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let total_entities = result.entity_reviews.len();

    let min_level = parse_risk_level(&args.min_risk);
    result.entity_reviews.retain(|r| r.risk_level >= min_level);
    result.entity_reviews.truncate(args.max_entities);

    let review_count = result.entity_reviews.len();

    if review_count == 0 {
        eprintln!("No entities at {} risk or above.", args.min_risk);
        std::process::exit(0);
    }

    let reduction = if total_entities > 0 {
        ((total_entities - review_count) as f64 / total_entities as f64 * 100.0) as u32
    } else {
        0
    };

    eprintln!(
        "Triaged {} entities -> {} for LLM review ({}% reduction)",
        total_entities, review_count, reduction
    );

    let client = match build_provider(&args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let mut reviews: Vec<EntityLlmReview> = Vec::new();

    for (i, entity) in result.entity_reviews.iter().enumerate() {
        eprint!(
            "  [{}/{}] Reviewing {} ... ",
            i + 1,
            review_count,
            entity.entity_name
        );

        match client.review_entity(entity).await {
            Ok(review) => {
                eprintln!("{}", format_verdict_inline(review.verdict));
                reviews.push(review);
            }
            Err(e) => {
                eprintln!("{}", format!("error: {}", e).red());
            }
        }
    }

    match args.format {
        OutputFormat::Terminal => print_terminal(&reviews),
        OutputFormat::Json => print_json(&reviews),
        OutputFormat::Markdown => print_markdown(&reviews),
    }
}

fn format_verdict_inline(verdict: LlmVerdict) -> String {
    match verdict {
        LlmVerdict::Approve => "approved".green().to_string(),
        LlmVerdict::Comment => "comment".yellow().to_string(),
        LlmVerdict::RequestChanges => "changes requested".red().bold().to_string(),
    }
}

fn print_terminal(reviews: &[EntityLlmReview]) {
    if reviews.is_empty() {
        return;
    }

    let total_tokens: u64 = reviews.iter().map(|r| r.tokens_used).sum();
    let changes_requested = reviews
        .iter()
        .filter(|r| r.verdict == LlmVerdict::RequestChanges)
        .count();
    let comments = reviews
        .iter()
        .filter(|r| r.verdict == LlmVerdict::Comment)
        .count();
    let approved = reviews
        .iter()
        .filter(|r| r.verdict == LlmVerdict::Approve)
        .count();

    println!(
        "\n{} {} entities reviewed ({} tokens)",
        "review".bold().cyan(),
        reviews.len(),
        total_tokens,
    );
    println!(
        "  {} approved, {} comments, {} changes requested",
        format!("{}", approved).green(),
        format!("{}", comments).yellow(),
        format!("{}", changes_requested).red(),
    );

    for review in reviews {
        let badge = match review.verdict {
            LlmVerdict::Approve => " APPROVE ".on_green().white().bold().to_string(),
            LlmVerdict::Comment => " COMMENT ".on_yellow().black().bold().to_string(),
            LlmVerdict::RequestChanges => {
                " CHANGES ".on_red().white().bold().to_string()
            }
        };

        println!(
            "\n  {} {} {}",
            badge,
            review.entity_name.bold(),
            format!("({})", review.file_path).dimmed(),
        );

        if !review.summary.is_empty() {
            println!("    {}", review.summary);
        }

        for issue in &review.issues {
            let sev = match issue.severity.as_str() {
                "error" => "error".red().bold().to_string(),
                "warning" => "warning".yellow().to_string(),
                _ => "info".dimmed().to_string(),
            };
            println!("    [{}] {}", sev, issue.description);
        }
    }

    println!();
}

fn print_json(reviews: &[EntityLlmReview]) {
    println!("{}", serde_json::to_string_pretty(reviews).unwrap());
}

fn print_markdown(reviews: &[EntityLlmReview]) {
    println!("# Code Review\n");

    let changes_requested = reviews
        .iter()
        .filter(|r| r.verdict == LlmVerdict::RequestChanges)
        .count();
    let comments = reviews
        .iter()
        .filter(|r| r.verdict == LlmVerdict::Comment)
        .count();
    let approved = reviews
        .iter()
        .filter(|r| r.verdict == LlmVerdict::Approve)
        .count();

    println!(
        "{} entities reviewed: {} approved, {} comments, {} changes requested\n",
        reviews.len(),
        approved,
        comments,
        changes_requested,
    );

    for review in reviews {
        let verdict_str = match review.verdict {
            LlmVerdict::Approve => "Approve",
            LlmVerdict::Comment => "Comment",
            LlmVerdict::RequestChanges => "Changes Requested",
        };

        println!(
            "## {} `{}` ({})\n",
            verdict_str, review.entity_name, review.file_path
        );

        if !review.summary.is_empty() {
            println!("{}\n", review.summary);
        }

        for issue in &review.issues {
            println!("- **{}**: {}", issue.severity, issue.description);
        }

        println!();
    }
}

async fn run_remote(args: ReviewArgs) {
    let remote = args.remote.as_ref().unwrap();
    let pr_number: u64 = match args.target.parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("{}", "error: target must be a PR number when using --remote".red());
            std::process::exit(1);
        }
    };

    let creds = match crate::config::require_credentials() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e.red());
            std::process::exit(1);
        }
    };

    let client = reqwest::Client::new();

    eprintln!(
        "Submitting review for {} PR #{}...",
        remote.bold(),
        pr_number
    );

    // POST /v1/review
    let body = serde_json::json!({
        "repo": remote,
        "pr_number": pr_number,
        "strategy": args.strategy,
    });

    let resp = client
        .post(format!("{}/v1/review", creds.api_url))
        .header("Authorization", format!("Bearer {}", creds.api_key))
        .json(&body)
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", format!("error: could not reach API: {e}").red());
            std::process::exit(1);
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        eprintln!("{}", format!("error: API returned {status}: {text}").red());
        std::process::exit(1);
    }

    let create_resp: serde_json::Value = resp.json().await.unwrap();
    let job_id = create_resp["id"].as_str().unwrap().to_string();

    // Poll GET /v1/review/{id}
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(args.timeout);
    let poll_interval = std::time::Duration::from_secs(2);

    loop {
        if std::time::Instant::now() > deadline {
            eprintln!("{}", format!("error: timed out after {}s", args.timeout).red());
            std::process::exit(1);
        }

        tokio::time::sleep(poll_interval).await;

        let poll = client
            .get(format!("{}/v1/review/{}", creds.api_url, job_id))
            .header("Authorization", format!("Bearer {}", creds.api_key))
            .send()
            .await;

        let poll_resp = match poll {
            Ok(r) => r,
            Err(e) => {
                eprintln!("  poll error: {e}");
                continue;
            }
        };

        let job: serde_json::Value = poll_resp.json().await.unwrap();
        let status = job["status"].as_str().unwrap_or("unknown");

        eprint!("\r  Status: {}    ", status);

        match status {
            "complete" => {
                eprintln!();
                print_remote_result(&args, &job);
                return;
            }
            "failed" => {
                eprintln!();
                let err = job["error"].as_str().unwrap_or("unknown error");
                eprintln!("{}", format!("error: review failed: {err}").red());
                std::process::exit(1);
            }
            _ => continue,
        }
    }
}

fn print_remote_result(args: &ReviewArgs, job: &serde_json::Value) {
    match args.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&job["result"]).unwrap());
        }
        OutputFormat::Markdown | OutputFormat::Terminal => {
            let result = &job["result"];
            let triage = &result["triage"];
            let timing = &result["timing"];

            let verdict = triage["verdict"].as_str().unwrap_or("unknown");
            let total_entities = triage["total_entities"].as_u64().unwrap_or(0);

            eprintln!(
                "Triage: {} entities, verdict: {}",
                total_entities,
                match verdict {
                    "approve" => verdict.green().to_string(),
                    "review" => verdict.yellow().to_string(),
                    _ => verdict.red().to_string(),
                }
            );

            let findings = result["findings"].as_array();
            let finding_count = findings.map(|f| f.len()).unwrap_or(0);

            if finding_count > 0 {
                println!("{} findings:", finding_count);
                for f in findings.unwrap() {
                    let severity = f["severity"].as_str().unwrap_or("info");
                    let description = f["description"].as_str().unwrap_or("");
                    let file = f["file"].as_str().unwrap_or("");
                    let sev_colored = match severity {
                        "error" | "critical" | "high" => format!("[{}]", severity).red().to_string(),
                        "warning" | "medium" => format!("[{}]", severity).yellow().to_string(),
                        _ => format!("[{}]", severity).dimmed().to_string(),
                    };
                    println!("  {} {} in {}", sev_colored, description, file.dimmed());
                }
            } else {
                println!("No findings.");
            }

            let triage_ms = timing["triage_ms"].as_u64().unwrap_or(0);
            let review_ms = timing["review_ms"].as_u64().unwrap_or(0);
            let total_ms = timing["total_ms"].as_u64().unwrap_or(0);
            eprintln!(
                "Timing: triage {}ms, review {}ms, total {}ms",
                triage_ms, review_ms, total_ms
            );
        }
    }
}

fn parse_scope(target: &str) -> DiffScope {
    if target.contains("..") {
        let parts: Vec<&str> = target.split("..").collect();
        DiffScope::Range {
            from: parts[0].to_string(),
            to: parts[1].to_string(),
        }
    } else {
        DiffScope::Commit {
            sha: target.to_string(),
        }
    }
}

fn parse_risk_level(s: &str) -> RiskLevel {
    match s.to_lowercase().as_str() {
        "critical" => RiskLevel::Critical,
        "high" => RiskLevel::High,
        "medium" => RiskLevel::Medium,
        _ => RiskLevel::Low,
    }
}
