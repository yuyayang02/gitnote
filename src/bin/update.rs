const UPDATE_API: &'static str = "http://localhost:3000/api/repo/update";

fn print_usage_and_exit() -> ! {
    eprintln!("Usage: update <refname> <oldrev> <newrev>");
    std::process::exit(1);
}

fn main() {
    let mut args = std::env::args().skip(1); // 跳过程序名

    let refname = args.next().unwrap_or_else(|| {
        eprintln!("Missing <refname>");
        print_usage_and_exit();
    });

    let oldrev = args.next().unwrap_or_else(|| {
        eprintln!("Missing <oldrev>");
        print_usage_and_exit();
    });

    let newrev = args.next().unwrap_or_else(|| {
        eprintln!("Missing <newrev>");
        print_usage_and_exit();
    });

    if args.next().is_some() {
        eprintln!("Too many arguments provided.");
        print_usage_and_exit();
    }

    const DEFAULT_REFNAME: &'static str = "refs/heads/main";

    // ✅ 只处理 main 分支的推送
    if refname != DEFAULT_REFNAME {
        println!("ℹ️ Skipping update hook: ref '{}' is not 'main'", refname);
        return;
    }

    let client = reqwest::blocking::Client::new();
    let res = client
        .post(UPDATE_API)
        .json(&serde_json::json!({
            "refname": refname,
            "oldrev": oldrev,
            "newrev": newrev,
        }))
        .send();

    match res {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            if !status.is_success() {
                // 获取 body 内容并输出
                eprintln!("❌ Push rejected");
                eprintln!("{} | {}", status, text.trim());
                std::process::exit(1);
            } else {
                println!("✅ Push successful");
                println!("{}", text.trim());
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to contact validation API: {}", e);
            std::process::exit(1);
        }
    }
}
