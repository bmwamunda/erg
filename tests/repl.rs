mod common;
use common::expect_repl_failure;
use common::expect_repl_success;

#[test]
fn exec_repl_helloworld() -> Result<(), ()> {
    expect_repl_success(
        "repl_hello",
        ["print! \"hello, world\"", "exit()"]
            .into_iter()
            .map(|x| x.to_string())
            .collect(),
    )
}

#[test]
fn exec_repl_def_func() -> Result<(), ()> {
    expect_repl_success(
        "repl_def",
        ["f i =", "i + 1", "", "x = f 2", "assert x == 3", "exit()"]
            .into_iter()
            .map(|x| x.to_string())
            .collect(),
    )
}

#[test]
fn exec_repl_for_loop() -> Result<(), ()> {
    expect_repl_success(
        "repl_for",
        ["for! 0..1, i =>", "print! i", "", "exit()"]
            .into_iter()
            .map(|line| line.to_string())
            .collect(),
    )
}

#[test]
fn exec_repl_auto_indent_dedent_check() -> Result<(), ()> {
    expect_repl_success(
        "repl_auto_indent_dedent",
        [
            "for! 0..0, i =>",
            "for! 0..0, j =>",
            "for! 0..0, k =>",
            "for! 0..0, l =>",
            "print! \"hi\"",
            "# l indent",
            "", // dedent l
            "# k indent",
            "", // dedent k
            "# j indent",
            "", // dedent j
            "# i indent and `for!` loop finished",
            "",
            "# main",
            "exit()",
        ]
        .into_iter()
        .map(|line| line.to_string())
        .collect(),
    )
}

#[test]
fn exec_repl_class_def() -> Result<(), ()> {
    expect_repl_success(
        "repl_auto_indent_dedent",
        [
            "C = Class()",
            "C.",
            "attr = 1",
            "",
            "print! C.attr",
            ":exit",
        ]
        .into_iter()
        .map(|line| line.to_string())
        .collect(),
    )
}

#[test]
fn exec_repl_class_def_with_deco() -> Result<(), ()> {
    expect_repl_success(
        "repl_auto_indent_dedent",
        [
            "@Inheritable",
            "C = Class{ x = Int }",
            "C.",
            "attr = 1",
            "",
            "print! C.attr",
            ":exit",
        ]
        .into_iter()
        .map(|line| line.to_string())
        .collect(),
    )
}

#[test]
fn exec_invalid_class_def() -> Result<(), ()> {
    expect_repl_failure(
        "repl_auto_indent_dedent",
        [
            "C = a Class() # Invalid but pass the expect block",
            "C.",
            "attr = 1",
            "",
            "print! C.attr",
            ":exit",
        ]
        .into_iter()
        .map(|line| line.to_string())
        .collect(),
        2,
    )
}

#[test]
fn exec_repl_invalid_indent() -> Result<(), ()> {
    expect_repl_failure(
        "repl_invalid_indent",
        [
            "a =",
            "    1",
            "2",
            "",
            "x =>",
            "1",
            "    print! \"hi\"",
            "",
            "exit()",
        ]
        .into_iter()
        .map(|x| x.to_string())
        .collect(),
        3,
    )
}

#[test]
fn exec_repl_invalid_def_after_the_at_sign() -> Result<(), ()> {
    expect_repl_failure(
        "repl_invalid_indent",
        ["@decorator", "a = 1", "exit()"]
            .into_iter()
            .map(|x| x.to_string())
            .collect(),
        1,
    )
}
