use leptos::prelude::*;

use super::types::ProcessingResult;
use pulldown_cmark::{Parser, Options, html};

#[derive(Clone, Copy, PartialEq, Eq)]
enum PaneView {
    F2P,
    P2P,
    Issue,
    PRFiles,
    PRTests,
}

impl PaneView {
    fn to_label(&self) -> &'static str {
        match self {
            PaneView::F2P => "F2P",
            PaneView::P2P => "P2P",
            PaneView::Issue => "Issue",
            PaneView::PRFiles => "PR Src",
            PaneView::PRTests => "PR Tests",
        }
    }
}

fn next_view(current: PaneView, allowed: &[PaneView]) -> PaneView {
    let idx = allowed.iter().position(|v| *v == current).unwrap_or(0);
    allowed[(idx + 1) % allowed.len()]
}

fn prev_view(current: PaneView, allowed: &[PaneView]) -> PaneView {
    let idx = allowed.iter().position(|v| *v == current).unwrap_or(0);
    allowed[(idx + allowed.len() - 1) % allowed.len()]
}

fn pr_url(result: Option<ProcessingResult>) -> String {
    if let Some(r) = result {
        if !r.pr_id.is_empty() {
            let repo = if !r.repo.is_empty() { r.repo } else { "swe-bench/SWE-bench".to_string() };
            return format!("https://github.com/{}/pull/{}", repo, r.pr_id);
        }
    }
    String::new()
}

fn issue_url(result: Option<ProcessingResult>) -> String {
    if let Some(r) = result {
        if !r.issue_id.is_empty() {
            let repo = if !r.repo.is_empty() { r.repo } else { "swe-bench/SWE-bench".to_string() };
            return format!("https://github.com/{}/issues/{}", repo, r.issue_id);
        }
    }
    String::new()
}

fn pr_files_url(result: Option<ProcessingResult>) -> String {
    if let Some(r) = result {
        if !r.pr_id.is_empty() {
            let repo = if !r.repo.is_empty() { r.repo } else { "swe-bench/SWE-bench".to_string() };
            return format!("https://github.com/{}/pull/{}/files", repo, r.pr_id);
        }
    }
    String::new()
}

#[component]
pub fn Playground(
    result: RwSignal<Option<ProcessingResult>>,
    fail_to_pass_tests: RwSignal<Vec<String>>,
    pass_to_pass_tests: RwSignal<Vec<String>>,
) -> impl IntoView {
    let right_allowed = vec![PaneView::Issue, PaneView::PRFiles, PaneView::PRTests];
    let left_top_allowed = vec![PaneView::Issue, PaneView::P2P, PaneView::F2P];
    let left_bottom_allowed = vec![PaneView::F2P, PaneView::P2P];

    let left_top = RwSignal::new(PaneView::Issue);
    let left_bottom = RwSignal::new(PaneView::F2P);
    let right = RwSignal::new(PaneView::PRFiles);

    let render_tests_list = move |tests: Vec<String>| {
        view! {
            <div class="h-full overflow-auto bg-white dark:bg-gray-800">
                <ul class="divide-y divide-gray-200 dark:divide-gray-600">
                    <For
                        each=move || tests.clone()
                        key=|name| name.clone()
                        children=move |name| {
                            view! {
                                <li class="px-3 py-1 text-sm text-gray-800 dark:text-gray-300 truncate">{name}</li>
                            }
                        }
                    />
                </ul>
            </div>
        }
    };

    let render_iframe = move |url: String| {
        view! {
            <div class="w-full h-full">
                <iframe
                    class="w-full h-full border-0"
                    src=url
                    referrerpolicy="no-referrer"
                />
            </div>
        }
    };

    let render_issue_like = move || {
        view! {
            <div class="h-full w-full overflow-auto bg-white dark:bg-gray-800 p-3">
                <div class="mb-3 border border-gray-200 dark:border-gray-600 rounded">
                    <div class="px-3 py-2 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600">
                        <span class="text-sm font-semibold text-gray-800 dark:text-gray-100">Problem Statement</span>
                    </div>
                    {let md = Memo::new(move |_| result.get().map(|r| r.problem_statement.clone()).unwrap_or_default());
                     let rendered = Memo::new(move |_| {
                        let mut opts = Options::empty();
                        opts.insert(Options::ENABLE_TABLES);
                        let binding = md.get();
                        let parser = Parser::new_ext(&binding, opts);
                        let mut html_out = String::new();
                        html::push_html(&mut html_out, parser);
                        html_out
                     });
                     view! { <div class="px-3 py-2 prose prose-sm dark:prose-invert max-w-none" inner_html=rendered.get()></div> }
                    }
                </div>
                <div>
                    <h3 class="text-sm font-semibold text-gray-900 dark:text-white mb-2">Conversation</h3>
                    <Show
                        when=move || result.get().map(|r| !r.conversation.is_empty()).unwrap_or(false)
                        fallback=move || view!{<div class="text-sm text-gray-500 dark:text-gray-400">No conversation.</div>}
                    >
                        <ul class="space-y-3">
                            <For
                                each=move || result.get().map(|r| r.conversation.clone()).unwrap_or_default()
                                key=|e| format!("{}-{}", e.author, e.timestamp)
                                children=move |e| {
                                    let content_memo = Memo::new(move |_| e.content.clone());
                                    let rendered_memo = Memo::new(move |_| {
                                        let mut opts = Options::empty();
                                        opts.insert(Options::ENABLE_TABLES);
                                        let binding = content_memo.get();
                                        let parser = Parser::new_ext(&binding, opts);
                                        let mut html_out = String::new();
                                        html::push_html(&mut html_out, parser);
                                        html_out
                                    });
                                    view! { 
                                        <li class="border border-gray-200 dark:border-gray-600 rounded">
                                            <div class="px-3 py-2 bg-gray-50 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600 flex items-center justify-between">
                                                <span class="text-xs font-medium text-gray-700 dark:text-gray-300">{e.author}</span>
                                                <span class="text-[10px] text-gray-500 dark:text-gray-400">{e.timestamp}</span>
                                            </div>
                                            <div class="px-3 py-2 prose prose-sm dark:prose-invert max-w-none" inner_html=rendered_memo.get()></div>
                                        </li>
                                    }
                                }
                            />
                        </ul>
                    </Show>
                </div>
            </div>
        }
    };

    // Simple unified diff rendering helpers
    let render_unified_diff = move |patch: String| -> AnyView {
        let lines: Vec<String> = patch.lines().map(|s| s.to_string()).collect();
        let mut current_file: Option<(String, String)> = None;
        let mut chunks: Vec<AnyView> = Vec::new();
        
        #[derive(Clone)]
        struct Row { prefix: char, text: String, left: Option<i64>, right: Option<i64>, is_header: bool }
        let mut buffer: Vec<Row> = Vec::new();

        let flush = |file: &Option<(String, String)>, buf: &mut Vec<Row>, out: &mut Vec<AnyView>| {
            if file.is_none() && buf.is_empty() { return; }
            let sanitize = |mut s: String| {
                if s.starts_with("a/") || s.starts_with("b/") { s = s[2..].to_string(); }
                s
            };
            let (old_name, new_name) = file.clone().unwrap_or_else(|| ("".to_string(), "".to_string()));
            let old_clean = sanitize(old_name);
            let new_clean = sanitize(new_name);
            let file_name = if old_clean.is_empty() || old_clean == new_clean { new_clean.clone() } else { format!("{} → {}", old_clean, new_clean) };
            let items = buf.iter().map(|row| {
                let ch = row.prefix;
                let (bg, prefix_class, border_class) = match ch {
                    '+' => ("bg-green-50 dark:bg-green-700/40", "text-green-700 dark:text-green-200", "border-l-2 border-green-400 dark:border-green-300"),
                    '-' => ("bg-red-50 dark:bg-red-700/40", "text-red-700 dark:text-red-200", "border-l-2 border-red-400 dark:border-red-300"),
                    '@' => ("bg-blue-100 dark:bg-sky-800/60", "text-blue-900 dark:text-sky-200", "border-l-2 border-sky-400 dark:border-sky-300"),
                    _ => ("bg-white dark:bg-gray-800", "text-gray-500 dark:text-gray-400", "border-l border-transparent"),
                };
                let line_text = row.text.clone();
                let left_num = row.left.map(|n| n.to_string()).unwrap_or_default();
                let right_num = row.right.map(|n| n.to_string()).unwrap_or_default();
                view! {
                    <div class=format!("grid grid-cols-[48px_48px_1fr] gap-2 px-2 py-0.5 text-xs font-mono {} {} {}", bg, border_class, if row.is_header {"mb-1"} else {""})>
                        <span class="text-right text-gray-400 dark:text-gray-500">{left_num}</span>
                        <span class="text-right text-gray-400 dark:text-gray-500">{right_num}</span>
                        <div class="flex items-start">
                            <span class=format!("mr-2 {}", prefix_class)>{ch}</span>
                            <span class="whitespace-pre-wrap text-gray-900 dark:text-gray-100">{line_text}</span>
                        </div>
                    </div>
                }.into_any()
            }).collect::<Vec<_>>();
            let expanded = RwSignal::new(true);
            out.push(view! {
                <div class="mb-3 border border-gray-200 dark:border-gray-600 rounded overflow-hidden bg-white dark:bg-gray-800">
                    <div class="px-3 py-1 text-xs bg-gray-100 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600 font-semibold truncate flex items-center justify-between text-gray-800 dark:text-gray-100">
                        <div class="truncate">{file_name}</div>
                        <button class="px-2 py-0.5 text-xs rounded bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600"
                            on:click=move |_| expanded.set(!expanded.get())>
                            {move || if expanded.get() { "Collapse".to_string() } else { "Expand".to_string() }}
                        </button>
                    </div>
                    <div class=move || if expanded.get() { "max-h-full overflow-auto".to_string() } else { "hidden".to_string() }>
                        {items.into_iter().collect_view()}
                    </div>
                </div>
            }.into_any());
            buf.clear();
        };

        let mut old_line: Option<i64> = None;
        let mut new_line: Option<i64> = None;

        for line in lines {
            if line.starts_with("diff --git ") {
                // New file section
                flush(&current_file, &mut buffer, &mut chunks);
                // Extract source and dest path (a/..., b/...)
                let parts: Vec<&str> = line.split_whitespace().collect();
                let a_name = if parts.len() >= 3 { parts[2].to_string() } else { String::new() };
                let b_name = if parts.len() >= 4 { parts[3].to_string() } else { String::new() };
                current_file = Some((a_name, b_name));
                old_line = None; new_line = None;
            } else if line.starts_with("index ") || line.starts_with("new file mode ") || line.starts_with("deleted file mode ") || line.starts_with("file mode ") || line.starts_with("similarity index ") || line.starts_with("rename from ") || line.starts_with("rename to ") {
                // Skip metadata lines
                continue;
            } else if line.starts_with("+++") || line.starts_with("---") {
                // Skip file header lines (must be checked before +/- branches)
                continue;
            } else if line.starts_with("@@ ") {
                // Parse hunk header @@ -a,b +c,d @@ optional
                let inner = line.trim_start_matches("@@ ");
                let inner = inner.trim_end_matches(" @@");
                let mut parts = inner.split(' ');
                let old_part = parts.next().unwrap_or(""); // -a,b
                let new_part = parts.next().unwrap_or(""); // +c,d
                let parse_range = |s: &str| -> (i64, i64) {
                    let s = s.trim_start_matches('-').trim_start_matches('+');
                    let mut it = s.split(',');
                    let start = it.next().unwrap_or("0").parse::<i64>().unwrap_or(0);
                    let cnt = it.next().unwrap_or("1").parse::<i64>().unwrap_or(1);
                    (start, cnt)
                };
                let (o_start, _) = parse_range(old_part);
                let (n_start, _) = parse_range(new_part);
                old_line = Some(o_start);
                new_line = Some(n_start);
                // Show header row without duplicate @@ and add a spacer after
                let header_text = inner.replace(old_part, &format!("{}", old_part)).replace(new_part, &format!("{}", new_part));
                buffer.push(Row { prefix: '@', text: header_text, left: None, right: None, is_header: true });
                buffer.push(Row { prefix: ' ', text: String::new(), left: None, right: None, is_header: false });
            } else if line.starts_with('+') {
                let text = line[1..].to_string();
                let ln = new_line;
                if let Some(n) = new_line { new_line = Some(n + 1); }
                buffer.push(Row { prefix: '+', text, left: None, right: ln, is_header: false });
            } else if line.starts_with('-') {
                let text = line[1..].to_string();
                let ln = old_line;
                if let Some(n) = old_line { old_line = Some(n + 1); }
                buffer.push(Row { prefix: '-', text, left: ln, right: None, is_header: false });
            } else {
                // context line
                let ln_l = old_line;
                let ln_r = new_line;
                if let Some(n) = old_line { old_line = Some(n + 1); }
                if let Some(n) = new_line { new_line = Some(n + 1); }
                buffer.push(Row { prefix: ' ', text: line, left: ln_l, right: ln_r, is_header: false });
            }
        }
        flush(&current_file, &mut buffer, &mut chunks);

        view! { <div class="h-full w-full overflow-auto p-2">{chunks.into_iter().collect_view()}</div> }.into_any()
    };

    let render_pr_files_diff = move || {
        let gold = result.get().map(|r| r.gold_patch).unwrap_or_default();
        render_unified_diff(gold)
    };

    let render_pr_tests_diff = move || {
        let test = result.get().map(|r| r.test_patch).unwrap_or_default();
        render_unified_diff(test)
    };

    let render_pane = move |title: String, view_signal: RwSignal<PaneView>, allowed: Vec<PaneView>| {
        let title_clone = title.clone();
        let allowed_prev = allowed.clone();
        let allowed_next = allowed.clone();
        view! {
            <div class="flex flex-col h-full overflow-hidden border border-gray-300 dark:border-gray-700 rounded">
                <div class="flex items-center justify-between px-2 py-1 bg-gray-200 dark:bg-gray-700 border-b border-gray-200 dark:border-gray-600">
                    <span class="text-xs font-semibold text-gray-800 dark:text-gray-100">
                        {move || view_signal.get().to_label().to_string()}
                    </span>
                    <div class="flex items-center gap-1">
                        <button class="px-2 py-0.5 text-xs rounded bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300"
                            on:click=move |_| view_signal.set(prev_view(view_signal.get(), &allowed_prev))>{"<"}</button>
                        <span class="text-xs text-gray-600 dark:text-gray-300 w-14 text-center truncate">{move || view_signal.get().to_label()}</span>
                        <button class="px-2 py-0.5 text-xs rounded bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300"
                            on:click=move |_| view_signal.set(next_view(view_signal.get(), &allowed_next))>{">"}</button>
                    </div>
                </div>
                <div class="flex-1 min-h-0 overflow-auto">
                    {move || {
                        match view_signal.get() {
                            PaneView::F2P => render_tests_list(fail_to_pass_tests.get()).into_any(),
                            PaneView::P2P => render_tests_list(pass_to_pass_tests.get()).into_any(),
                            PaneView::Issue => render_issue_like().into_any(),
                            PaneView::PRFiles => render_pr_files_diff().into_any(),
                            PaneView::PRTests => render_pr_tests_diff().into_any(),
                        }
                    }}
                </div>
            </div>
        }
    };

    view! {
            <div class="h-full w-full bg-white dark:bg-gray-800">
            <div class="grid grid-cols-2 gap-1 h-full min-h-0">
                <div class="col-span-1 grid grid-rows-2 gap-1 h-full min-h-0">
                    {render_pane("Left Top".to_string(), left_top, left_top_allowed.clone())}
                    {render_pane("Left Bottom".to_string(), left_bottom, left_bottom_allowed.clone())}
                </div>
                <div class="col-span-1 h-full min-h-0">
                    {render_pane("Right".to_string(), right, right_allowed.clone())}
                </div>
            </div>
        </div>
    }
}

