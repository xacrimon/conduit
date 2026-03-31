pub fn script() -> maud::Markup {
    maud::html! {
        script defer src="/assets/lib/ace-1.43.4/ace.js" {}
    }
}

pub fn readonly(editor_id: &str, mode: &str) -> maud::Markup {
    let js = format!(
        r#"
            addEventListener("DOMContentLoaded", (_) => {{
                let editor = ace.edit("{editor_id}");
                editor.setTheme("ace/theme/github");
                editor.session.setMode("ace/mode/{mode}");
                editor.setReadOnly(true);
                editor.setShowPrintMargin(false);
                editor.renderer.setShowGutter(true);
            }})
        "#,
    );

    maud::html! {
        script { (maud::PreEscaped(js)) }
    }
}

pub fn enable(editor_id: &str, input_id: &str) -> maud::Markup {
    let js = format!(
        r#"
            addEventListener("DOMContentLoaded", (_) => {{
                let editor = ace.edit("{editor_id}");
                let input = document.getElementById("{input_id}");
                editor.on("change", () => input.value = editor.getValue());
            }})
        "#,
    );

    maud::html! {
        script { (maud::PreEscaped(js)) }
    }
}

pub fn infer_mode(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "rust",
        "js" => "javascript",
        "ts" => "typescript",
        "py" => "python",
        "go" => "golang",
        "c" | "h" => "c_cpp",
        "cpp" | "cc" | "cxx" | "hpp" => "c_cpp",
        "java" => "java",
        "rb" => "ruby",
        "php" => "php",
        "html" | "htm" => "html",
        "css" => "css",
        "json" => "json",
        "xml" => "xml",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "md" | "markdown" => "markdown",
        "sh" | "bash" => "sh",
        "sql" => "sql",
        _ => "text",
    }
}
