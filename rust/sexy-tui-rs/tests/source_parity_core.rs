use sexy_tui_rs::{
    fuzzy_filter, fuzzy_match,
    keybindings::{KeybindingsManager, TUI_KEYBINDINGS},
    terminal_colors::{parse_osc11_background_color, parse_terminal_color_scheme_report, RgbColor},
    terminal_image::is_image_line,
    utils::{normalize_terminal_output, truncate_to_width, visible_width, wrap_text_with_ansi},
    word_navigation::{find_word_backward, find_word_forward},
};

#[test]
fn fuzzy_match_source_parity() {
    let result = fuzzy_match("", "anything");
    assert!(result.matches);
    assert_eq!(result.score, 0.0);

    assert!(!fuzzy_match("longquery", "short").matches);
    assert!(fuzzy_match("test", "test").score < 0.0);
    assert!(fuzzy_match("abc", "aXbXc").matches);
    assert!(!fuzzy_match("abc", "cba").matches);
    assert!(fuzzy_match("ABC", "abc").matches);
    assert!(fuzzy_match("abc", "ABC").matches);

    let consecutive = fuzzy_match("foo", "foobar");
    let scattered = fuzzy_match("foo", "f_o_o_bar");
    assert!(consecutive.matches && scattered.matches);
    assert!(consecutive.score < scattered.score);

    let at_boundary = fuzzy_match("fb", "foo-bar");
    let not_at_boundary = fuzzy_match("fb", "afbx");
    assert!(at_boundary.matches && not_at_boundary.matches);
    assert!(at_boundary.score < not_at_boundary.score);

    assert!(fuzzy_match("codex52", "gpt-5.2-codex").matches);
}

#[test]
fn fuzzy_filter_source_parity() {
    let items = vec!["apple", "banana", "cherry"];
    assert_eq!(fuzzy_filter(&items, "", |x| x.to_string()), items);
    let result = fuzzy_filter(&items, "an", |x| x.to_string());
    assert!(result.contains(&"banana"));
    assert!(!result.contains(&"apple"));
    assert!(!result.contains(&"cherry"));

    let ranked = vec!["a_p_p", "app", "application"];
    assert_eq!(fuzzy_filter(&ranked, "app", |x| x.to_string())[0], "app");

    let exact = vec!["clone", "cl"];
    assert_eq!(
        fuzzy_filter(&exact, "cl", |x| x.to_string()),
        vec!["cl", "clone"]
    );

    #[derive(Clone, Debug, PartialEq)]
    struct Item {
        name: &'static str,
        id: u8,
    }
    let custom = vec![
        Item { name: "foo", id: 1 },
        Item { name: "bar", id: 2 },
        Item {
            name: "foobar",
            id: 3,
        },
    ];
    let filtered = fuzzy_filter(&custom, "foo", |item| item.name.to_string());
    assert_eq!(filtered.len(), 2);
    assert!(filtered.iter().any(|r| r.name == "foo"));
    assert!(filtered.iter().any(|r| r.name == "foobar"));

    #[derive(Clone, Debug, PartialEq)]
    struct Model {
        id: &'static str,
        provider: &'static str,
    }
    let model = Model {
        id: "gpt-5.5",
        provider: "openai-codex",
    };
    assert_eq!(
        fuzzy_filter(&[model.clone()], "openai-codex/gpt-5.5", |m| format!(
            "{} {}",
            m.id, m.provider
        )),
        vec![model]
    );
}

#[test]
fn keybindings_source_parity() {
    let keybindings = KeybindingsManager::new(TUI_KEYBINDINGS.clone());
    assert_eq!(
        keybindings.get_keys("tui.input.newLine"),
        vec!["shift+enter", "ctrl+j"]
    );
    assert!(keybindings.matches("\n", "tui.input.newLine"));
    assert!(keybindings.matches("\x1b[106;5u", "tui.input.newLine"));

    let mut rebound = KeybindingsManager::new(TUI_KEYBINDINGS.clone());
    rebound.set_user_bindings(vec![("tui.input.submit", vec!["enter", "ctrl+enter"])]);
    assert_eq!(
        rebound.get_keys("tui.input.submit"),
        vec!["enter", "ctrl+enter"]
    );
    assert_eq!(rebound.get_keys("tui.select.confirm"), vec!["enter"]);

    let mut reused = KeybindingsManager::new(TUI_KEYBINDINGS.clone());
    reused.set_user_bindings(vec![("tui.select.up", vec!["up", "ctrl+p"])]);
    assert_eq!(reused.get_keys("tui.select.up"), vec!["up", "ctrl+p"]);
    assert_eq!(reused.get_keys("tui.editor.cursorUp"), vec!["up"]);

    let mut conflicts = KeybindingsManager::new(TUI_KEYBINDINGS.clone());
    conflicts.set_user_bindings(vec![
        ("tui.input.submit", vec!["ctrl+x"]),
        ("tui.select.confirm", vec!["ctrl+x"]),
    ]);
    let conflict_list = conflicts.get_conflicts();
    assert_eq!(conflict_list.len(), 1);
    assert_eq!(conflict_list[0].key, "ctrl+x");
    assert_eq!(
        conflict_list[0].keybindings,
        vec!["tui.input.submit", "tui.select.confirm"]
    );
    assert_eq!(
        conflicts.get_keys("tui.editor.cursorLeft"),
        vec!["left", "ctrl+b"]
    );
}

#[test]
fn terminal_image_line_detection_source_parity() {
    assert!(is_image_line(
        "\x1b]1337;File=size=100,100;inline=1:base64encodeddata==\x07"
    ));
    assert!(is_image_line(
        "Some text \x1b]1337;File=size=100,100;inline=1:base64data==\x07 more text"
    ));
    assert!(is_image_line(
        "Text before image...\x1b]1337;File=inline=1:verylongbase64data==...text after"
    ));
    assert!(is_image_line(
        "Regular text ending with \x1b]1337;File=inline=1:base64data==\x07"
    ));
    assert!(is_image_line("\x1b]1337;File=:\x07"));
    assert!(is_image_line(
        "\x1b_Ga=T,f=100,t=f,d=base64data...\x1b\\\x1b_Gm=i=1;\x1b\\"
    ));
    assert!(is_image_line(
        "Output: \x1b_Ga=T,f=100;data...\x1b\\\x1b_Gm=i=1;\x1b\\"
    ));
    assert!(is_image_line(
        "  \x1b_Ga=T,f=100...\x1b\\\x1b_Gm=i=1;\x1b\\  "
    ));
    assert!(is_image_line(&format!(
        "Text prefix \x1b]1337;File=size=800,600;inline=1:{} suffix",
        "A".repeat(300_000)
    )));
    assert!(is_image_line(
        "Read image file [image/jpeg]\x1b]1337;File=inline=1:base64data==\x07"
    ));
    assert!(is_image_line(
        "\x1b[31mError output \x1b]1337;File=inline=1:image==\x07"
    ));
    assert!(is_image_line(
        "\x1b_Ga=T,f=100:data...\x1b\\\x1b_Gm=i=1;\x1b\\\x1b[0m reset"
    ));
    assert!(!is_image_line(
        "This is just a regular text line without any escape sequences"
    ));
    assert!(!is_image_line(
        "\x1b[31mRed text\x1b[0m and \x1b[32mgreen text\x1b[0m"
    ));
    assert!(!is_image_line("\x1b[1A\x1b[2KLine cleared and moved up"));
    assert!(!is_image_line(
        "Some text with ]1337;File but missing ESC at start"
    ));
    assert!(!is_image_line("Some text with _G but missing ESC at start"));
    assert!(!is_image_line(""));
    assert!(!is_image_line("\n"));
    assert!(!is_image_line("\n\n"));
    assert!(is_image_line(
        "Kitty: \x1b_Ga=T...\x1b\\\x1b_Gm=i=1;\x1b\\ iTerm2: \x1b]1337;File=inline=1:data==\x07"
    ));
    assert!(is_image_line(
        "Start \x1b]1337;File=img1==\x07 middle \x1b]1337;File=img2==\x07 end"
    ));
    assert!(!is_image_line("/path/to/File_1337_backup/image.jpg"));
}

#[test]
fn terminal_color_source_parity() {
    assert_eq!(
        parse_osc11_background_color("\x1b]11;rgb:0000/8000/ffff\x07"),
        Some(RgbColor {
            r: 0,
            g: 128,
            b: 255
        })
    );
    assert_eq!(
        parse_osc11_background_color("\x1b]11;#ffffff\x1b\\"),
        Some(RgbColor {
            r: 255,
            g: 255,
            b: 255
        })
    );
    assert_eq!(
        parse_osc11_background_color("\x1b]11;#000000\x07"),
        Some(RgbColor { r: 0, g: 0, b: 0 })
    );
    assert_eq!(parse_osc11_background_color("x\x1b]11;#ffffff\x07"), None);
    assert_eq!(parse_osc11_background_color("\x1b]10;#ffffff\x07"), None);
    assert_eq!(parse_osc11_background_color("\x1b]11;#ffffff\x07x"), None);

    assert_eq!(
        parse_terminal_color_scheme_report("\x1b[?997;1n"),
        Some("dark".into())
    );
    assert_eq!(
        parse_terminal_color_scheme_report("\x1b[?997;2n"),
        Some("light".into())
    );
    assert_eq!(parse_terminal_color_scheme_report("\x1b[?997;3n"), None);
    assert_eq!(parse_terminal_color_scheme_report("\x1b[?996n"), None);
    assert_eq!(parse_terminal_color_scheme_report("x\x1b[?997;1n"), None);
}

#[test]
fn word_navigation_source_parity() {
    assert_eq!(find_word_backward("hello world", 11, None), 6);
    assert_eq!(find_word_backward("hello world", 6, None), 0);
    assert_eq!(find_word_backward("foo.bar", 7, None), 4);
    assert_eq!(find_word_backward("foo.bar", 4, None), 3);
    assert_eq!(find_word_backward("foo.bar", 3, None), 0);
    assert_eq!(find_word_backward("foo:bar", 7, None), 4);
    assert_eq!(find_word_backward("path/to/file", 12, None), 8);
    assert_eq!(find_word_backward("path/to/file", 8, None), 7);
    assert_eq!(find_word_backward("path/to/file", 7, None), 5);
    assert_eq!(find_word_backward("path/to/file", 5, None), 4);
    assert_eq!(find_word_backward("path/to/file", 4, None), 0);
    assert_eq!(find_word_backward("  hello  ", 9, None), 2);
    assert_eq!(find_word_backward("foo...bar", 9, None), 6);
    assert_eq!(find_word_backward("foo...bar", 6, None), 3);
    assert_eq!(find_word_backward("foo...bar", 3, None), 0);
    assert_eq!(find_word_backward("hello", 0, None), 0);

    assert_eq!(find_word_forward("hello world", 0, None), 5);
    assert_eq!(find_word_forward("hello world", 5, None), 11);
    assert_eq!(find_word_forward("foo.bar", 0, None), 3);
    assert_eq!(find_word_forward("foo.bar", 3, None), 4);
    assert_eq!(find_word_forward("foo.bar", 4, None), 7);
    assert_eq!(find_word_forward("path/to/file", 0, None), 4);
    assert_eq!(find_word_forward("path/to/file", 4, None), 5);
    assert_eq!(find_word_forward("path/to/file", 5, None), 7);
    assert_eq!(find_word_forward("path/to/file", 7, None), 8);
    assert_eq!(find_word_forward("path/to/file", 8, None), 12);
    assert_eq!(find_word_forward("  hello  ", 0, None), 7);
    assert_eq!(find_word_forward("  hello  ", 7, None), 9);
    assert_eq!(find_word_forward("foo...bar", 0, None), 3);
    assert_eq!(find_word_forward("foo...bar", 3, None), 6);
    assert_eq!(find_word_forward("foo...bar", 6, None), 9);
    assert_eq!(find_word_forward("hello", 5, None), 5);
}

#[test]
fn visible_width_unicode_source_parity() {
    assert_eq!(visible_width("\t\x1b[31m界\x1b[0m"), 5);
    assert_eq!(visible_width("\x1b[31mé\x1b[0m"), 1);
    assert_eq!(visible_width("ำ"), 1);
    assert_eq!(visible_width("ຳ"), 1);
    assert_eq!(visible_width("กำ"), 2);
    assert_eq!(visible_width("ກຳ"), 2);
    assert_eq!(normalize_terminal_output("ำ"), "ํา");
    assert_eq!(normalize_terminal_output("ຳ"), "ໍາ");
    assert_eq!(
        visible_width(&normalize_terminal_output("ำabc")),
        visible_width("ำabc")
    );
    assert_eq!(
        visible_width(&normalize_terminal_output("ຳabc")),
        visible_width("ຳabc")
    );

    assert_eq!(visible_width("🇨"), 2);
    assert_eq!(visible_width("      - 🇨"), 10);
    for cp in 0x1f1e6..=0x1f1ff {
        let s = char::from_u32(cp).unwrap().to_string();
        assert_eq!(visible_width(&s), 2, "regional indicator U+{cp:X}");
    }
    for sample in [
        "🇯🇵",
        "🇺🇸",
        "🇬🇧",
        "🇨🇳",
        "🇩🇪",
        "🇫🇷",
        "👍",
        "👍🏻",
        "✅",
        "⚡",
        "⚡️",
        "👨",
        "👨‍💻",
        "🏳️‍🌈",
    ] {
        assert_eq!(visible_width(sample), 2, "{sample}");
    }
}

#[test]
fn truncate_and_wrap_source_parity() {
    let text = "🙂界".repeat(100_000);
    let truncated = truncate_to_width(&text, 40, Some("…"));
    assert!(visible_width(&truncated) <= 40);
    assert!(truncated.ends_with("…\x1b[0m"));

    let styled = format!("\x1b[31m{}\x1b[0m", "hello ".repeat(1000));
    let truncated = truncate_to_width(&styled, 20, Some("…"));
    assert!(visible_width(&truncated) <= 20);
    assert!(truncated.contains("\x1b[31m"));
    assert!(truncated.ends_with("\x1b[0m…\x1b[0m"));

    assert!(
        visible_width(&truncate_to_width(
            &format!("abc\x1bnot-ansi {}", "🙂".repeat(1000)),
            20,
            Some("…")
        )) <= 20
    );
    assert_eq!(truncate_to_width("abcdef", 1, Some("🙂")), "");
    assert_eq!(
        truncate_to_width("abcdef", 2, Some("🙂")),
        "\x1b[0m🙂\x1b[0m"
    );
    assert_eq!(truncate_to_width("a", 2, Some("🙂")), "a");
    assert_eq!(truncate_to_width("界", 2, Some("🙂")), "界");
    let no_ellipsis = truncate_to_width(&format!("\x1b[31m{}", "hello".repeat(100)), 10, Some(""));
    assert!(visible_width(&no_ellipsis) <= 10);
    assert!(no_ellipsis.ends_with("\x1b[0m"));

    let wrapped = wrap_text_with_ansi("      - 🇨", 9);
    assert_eq!(wrapped.len(), 2);
    assert_eq!(visible_width(&wrapped[0]), 7);
    assert_eq!(visible_width(&wrapped[1]), 2);
}
