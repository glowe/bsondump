mod tests {
    use std::io::{Read, Write};
    use std::process::Stdio;

    use tempfile::NamedTempFile;

    use test_bin;

    const SAMPLE_BSON: &[u8; 283] = include_bytes!("testdata/sample.bson");
    const SAMPLE_JSON: &[u8; 575] = include_bytes!("testdata/sample.json");

    #[test]
    fn from_stdin_to_stdout() {
        let mut child = test_bin::get_test_bin("bsondump")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to spawn process");

        let mut stdin = child.stdin.take().expect("Failed to open stdin");

        std::thread::spawn(move || {
            stdin
                .write_all(SAMPLE_BSON)
                .expect("Failed to write to stdin");
        });

        let output = child.wait_with_output().expect("Failed to read stdout");
        assert_eq!(&output.stdout, SAMPLE_JSON);
    }

    #[test]
    fn from_stdin_to_file() {
        let out_file = NamedTempFile::new().expect("Failed to create temporary out file");

        let mut child = test_bin::get_test_bin("bsondump")
            .args([
                "--outFile",
                out_file.path().to_str().expect("Failed get path"),
            ])
            .stdin(Stdio::piped())
            .spawn()
            .expect("failed to spawn process");

        let mut stdin = child.stdin.take().expect("Failed to open stdin");

        std::thread::spawn(move || {
            stdin
                .write_all(SAMPLE_BSON)
                .expect("Failed to write to stdin");
        });

        child.wait().expect("Failed to write");

        let mut file = std::fs::File::open(out_file.path()).expect("Failed to open out file");
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).expect("Failed to read out file");
        assert_eq!(buf, SAMPLE_JSON);
    }

    #[test]
    fn from_file_with_named_argument_to_stdout() {
        let output = test_bin::get_test_bin("bsondump")
            .args(["--bsonFile", "tests/testdata/sample.bson"])
            .stdout(Stdio::piped())
            .output()
            .expect("failed to read process output");

        assert_eq!(&output.stdout, SAMPLE_JSON);
    }

    #[test]
    fn from_file_with_positional_argument_to_stdout() {
        todo!();
    }

    #[test]
    fn from_file_with_named_argument_to_file() {
        let out_file = NamedTempFile::new().expect("Failed to create temporary out file");

        let mut child = test_bin::get_test_bin("bsondump")
            .args(["--bsonFile", "tests/testdata/sample.bson"])
            .args([
                "--outFile",
                out_file.path().to_str().expect("Failed get path"),
            ])
            .spawn()
            .expect("failed to read process output");

        child.wait().expect("Failed to wait for process");

        let mut file = std::fs::File::open(out_file.path()).expect("Failed to open out file");
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).expect("Failed to read out file");
        assert_eq!(buf, SAMPLE_JSON);
    }

    #[test]
    fn from_file_with_positional_argument_to_file() {
        todo!();
    }

    #[test]
    fn bsondump_max_bson_size() {
        todo!();
    }
}
