use payments_engine::process_csv;
use std::io::{self, Cursor, Read};

#[test]
fn csv_input_split_across_read_boundaries_is_processed_correctly() {
    let input = concat!(
        "type,client,tx,amount\n",
        "deposit,8,1000,7.1250\n",
        "withdrawal,8,1001,2.0000\n",
    );
    // A limit of three bytes deliberately cuts through headers, fields, and
    // row boundaries instead of returning complete CSV records.
    let reader = ChunkedReader::new(input.as_bytes(), 3);
    let mut output = Vec::new();

    process_csv(reader, &mut output).expect("chunked input should be valid");

    assert_eq!(
        String::from_utf8(output).expect("output should be UTF-8"),
        concat!(
            "client,available,held,total,locked\n",
            "8,5.1250,0.0000,5.1250,false\n",
        )
    );
}

#[test]
fn twenty_thousand_withdrawals_produce_an_exact_balance() {
    const WITHDRAWAL_COUNT: u32 = 20_000;
    let input = RepeatedWithdrawalReader::new(WITHDRAWAL_COUNT);
    let mut output = Vec::new();

    process_csv(input, &mut output).expect("generated input should be valid");

    assert_eq!(
        String::from_utf8(output).expect("output should be UTF-8"),
        concat!(
            "client,available,held,total,locked\n",
            "1,998.0000,0.0000,998.0000,false\n",
        )
    );
}

// Generates each withdrawal only when the CSV reader requests more input.
struct RepeatedWithdrawalReader {
    current_row: Cursor<Vec<u8>>,
    next_withdrawal_tx: u32,
    final_withdrawal_tx: u32,
}

impl RepeatedWithdrawalReader {
    fn new(withdrawal_count: u32) -> Self {
        Self {
            current_row: Cursor::new(
                concat!("type,client,tx,amount\n", "deposit,1,1,1000.0000\n",)
                    .as_bytes()
                    .to_vec(),
            ),
            next_withdrawal_tx: 2,
            final_withdrawal_tx: withdrawal_count + 1,
        }
    }

    fn prepare_next_row(&mut self) -> bool {
        if self.next_withdrawal_tx > self.final_withdrawal_tx {
            return false;
        }

        let transaction = self.next_withdrawal_tx;
        self.next_withdrawal_tx += 1;
        self.current_row = Cursor::new(format!("withdrawal,1,{transaction},0.0001\n").into_bytes());
        true
    }
}

impl Read for RepeatedWithdrawalReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }

        loop {
            let bytes_read = self.current_row.read(buffer)?;
            if bytes_read > 0 || !self.prepare_next_row() {
                return Ok(bytes_read);
            }
        }
    }
}

struct ChunkedReader<R> {
    inner: R,
    chunk_size: usize,
}

impl<R> ChunkedReader<R> {
    fn new(inner: R, chunk_size: usize) -> Self {
        Self { inner, chunk_size }
    }
}

impl<R: Read> Read for ChunkedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let length = buffer.len().min(self.chunk_size);
        self.inner.read(&mut buffer[..length])
    }
}
