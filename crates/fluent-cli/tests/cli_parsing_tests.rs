use fluent_cli::cli_builder::build_cli;

#[test]
fn pipeline_parses_new_args() {
    let app = build_cli();
    let m = app
        .try_get_matches_from(vec![
            "fluent",
            "pipeline",
            "--file",
            "p.yaml",
            "--input",
            "hello",
            "--run-id",
            "abc",
            "--force-fresh",
            "--json",
        ])
        .expect("should parse");
    let (sub, sm) = m.subcommand().expect("pipeline");
    assert_eq!(sub, "pipeline");
    assert_eq!(sm.get_one::<String>("file").map(|s| s.as_str()), Some("p.yaml"));
    assert_eq!(sm.get_one::<String>("input").map(|s| s.as_str()), Some("hello"));
    assert!(sm.get_flag("force_fresh"));
    assert!(sm.get_flag("json"));
}
