mod tests {
    use std::io::Write;
    use std::process::Stdio;

    use test_bin;

    #[test]
    fn from_stdin_to_stdout() {
        let mut child = test_bin::get_test_bin("bsondump")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn process");
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        let sample_bson = include_bytes!("testdata/sample.bson");
        std::thread::spawn(move || {
            stdin.write_all(sample_bson).expect("Failed to write to stdin");
        });

        let sample_json = include_str!("testdata/sample.json");
        let output = child.wait_with_output().expect("Failed to read stdout");
        assert_eq!(String::from_utf8_lossy(&output.stdout), sample_json);
        assert!(String::from_utf8_lossy(&output.stderr).ends_with("4 objects found\n"));
    }

    #[test]
    fn from_stdin_to_file() {}

    #[test]
    fn from_file_with_named_argument_to_stdout() {}

    #[test]
    fn from_file_with_positional_argument_to_stdout() {}

    #[test]
    fn from_file_with_named_argument_to_file() {}

    #[test]
    fn from_file_with_positional_argument_to_file() {}

    #[test]
    fn bsondump_max_bson_size() {}
}
