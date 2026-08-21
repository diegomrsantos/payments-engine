use payments_engine::{ProcessError, process_csv};
use std::fmt::Write as _;
use std::io::{self, Read, Write};

const READ_FAILURE: &str = "controlled reader failure";
const WRITE_FAILURE: &str = "controlled writer failure";
const FLUSH_FAILURE: &str = "controlled flush failure";
const PARTIAL_HEADER: &[u8] = b"client,avail";

#[test]
fn a_reader_failure_before_headers_is_reported_without_output() {
    let reader = ReaderThatFailsAfter::new(b"", 0);
    let mut output = Vec::new();

    let error = process_csv(reader, &mut output)
        .expect_err("the header read should report the reader failure");

    let chained_source = std::error::Error::source(&error)
        .expect("the processing error should retain its reader source");
    assert!(chained_source.to_string().contains(READ_FAILURE));
    match error {
        ProcessError::ReadHeaders(source) => assert_csv_io_failure(source, READ_FAILURE),
        other => panic!("expected a header read error, got {other}"),
    }
    assert!(output.is_empty(), "a header failure must not write output");
}

#[test]
fn a_reader_failure_during_rows_is_reported_without_output() {
    let input = concat!(
        "type,client,tx,amount\n",
        "deposit,1,100,2.0000\n",
        "deposit,1",
    );
    let reader = ReaderThatFailsAfter::new(input.as_bytes(), input.len());
    let mut output = Vec::new();

    let error = process_csv(reader, &mut output)
        .expect_err("the third row should report the reader failure");

    match error {
        ProcessError::ReadRow { row: 3, source } => {
            assert_csv_io_failure(source, READ_FAILURE);
        }
        other => panic!("expected a row 3 read error, got {other}"),
    }
    assert!(
        output.is_empty(),
        "a row read failure must not write account output"
    );
}

#[test]
fn writer_failure_during_row_serialization_is_reported_as_write_with_partial_output() {
    let input = deposits_that_fill_the_csv_writer_buffer();
    let mut writer = WriterThatFailsAfter::new(PARTIAL_HEADER.len());

    let error = process_csv(input.as_bytes(), &mut writer)
        .expect_err("writing the account rows should fail");

    match error {
        ProcessError::Write(source) => assert_csv_io_failure(source, WRITE_FAILURE),
        other => panic!("expected an account CSV write error, got {other}"),
    }
    assert_eq!(writer.written(), PARTIAL_HEADER);
}

#[test]
fn buffered_writer_failure_during_final_flush_is_reported_as_flush() {
    let input = concat!("type,client,tx,amount\n", "deposit,4,399,1.2500\n",);
    let mut writer = WriterThatFailsAfter::new(PARTIAL_HEADER.len());

    let error = process_csv(input.as_bytes(), &mut writer)
        .expect_err("draining the final CSV buffer should fail");

    match error {
        ProcessError::Flush(source) => assert_io_failure(source, WRITE_FAILURE),
        other => panic!("expected a final CSV flush error, got {other}"),
    }
    assert_eq!(writer.written(), PARTIAL_HEADER);
}

#[test]
fn a_flush_failure_is_reported_after_complete_output_was_written() {
    let input = concat!("type,client,tx,amount\n", "deposit,4,400,1.2500\n",);
    let expected = concat!(
        "client,available,held,total,locked\n",
        "4,1.2500,0.0000,1.2500,false\n",
    );
    let mut writer = WriterThatFailsOnFlush::default();

    let error = process_csv(input.as_bytes(), &mut writer)
        .expect_err("flushing the complete account CSV should fail");

    match error {
        ProcessError::Flush(source) => assert_io_failure(source, FLUSH_FAILURE),
        other => panic!("expected an account CSV flush error, got {other}"),
    }
    assert_eq!(writer.written(), expected.as_bytes());
}

fn deposits_that_fill_the_csv_writer_buffer() -> String {
    // Enough rows fill the CSV writer's internal buffer, forcing the failure
    // during row serialization instead of the explicit final flush.
    let mut input = String::from("type,client,tx,amount\n");
    for client in 1_u16..=400 {
        writeln!(input, "deposit,{client},{client},1.0000")
            .expect("writing the input story should succeed");
    }
    input
}

fn assert_csv_io_failure(source: csv::Error, message: &str) {
    match source.into_kind() {
        csv::ErrorKind::Io(source) => assert_io_failure(source, message),
        other => panic!("expected an I/O source error, got {other:?}"),
    }
}

fn assert_io_failure(source: io::Error, message: &str) {
    assert_eq!(source.kind(), io::ErrorKind::Other);
    assert_eq!(source.to_string(), message);
}

struct ReaderThatFailsAfter<'a> {
    input: &'a [u8],
    position: usize,
    fail_after: usize,
}

impl<'a> ReaderThatFailsAfter<'a> {
    fn new(input: &'a [u8], fail_after: usize) -> Self {
        assert!(fail_after <= input.len());
        Self {
            input,
            position: 0,
            fail_after,
        }
    }
}

impl Read for ReaderThatFailsAfter<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.position >= self.fail_after {
            return Err(io::Error::other(READ_FAILURE));
        }

        let end = self.fail_after.min(self.position + buffer.len());
        let length = end - self.position;
        buffer[..length].copy_from_slice(&self.input[self.position..end]);
        self.position = end;
        Ok(length)
    }
}

struct WriterThatFailsAfter {
    written: Vec<u8>,
    remaining: usize,
}

impl WriterThatFailsAfter {
    fn new(limit: usize) -> Self {
        Self {
            written: Vec::new(),
            remaining: limit,
        }
    }

    fn written(&self) -> &[u8] {
        &self.written
    }
}

impl Write for WriterThatFailsAfter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            return Err(io::Error::other(WRITE_FAILURE));
        }

        let length = buffer.len().min(self.remaining);
        self.written.extend_from_slice(&buffer[..length]);
        self.remaining -= length;
        Ok(length)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct WriterThatFailsOnFlush {
    written: Vec<u8>,
}

impl WriterThatFailsOnFlush {
    fn written(&self) -> &[u8] {
        &self.written
    }
}

impl Write for WriterThatFailsOnFlush {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.written.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other(FLUSH_FAILURE))
    }
}
