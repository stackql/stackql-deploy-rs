//! Manual test: REPLACE ... RETURNING over pgwire
//!
//! Requires a running stackql server on localhost:5444 with
//! databricks_account provider configured.
//!
//! Run with:
//!   cargo test --test pgwire_replace_test -- --nocapture --ignored

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;

fn read_byte(stream: &mut TcpStream) -> u8 {
    let mut buf = [0u8; 1];
    stream.read_exact(&mut buf).unwrap();
    buf[0]
}

fn read_i32(stream: &mut TcpStream) -> i32 {
    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).unwrap();
    i32::from_be_bytes(buf)
}

fn read_bytes(stream: &mut TcpStream, n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n];
    stream.read_exact(&mut buf).unwrap();
    buf
}

fn parse_error_fields(data: &[u8]) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    let mut pos = 0;
    while pos < data.len() {
        let field_type = data[pos];
        if field_type == 0 {
            break;
        }
        pos += 1;
        let end = data[pos..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(data.len() - pos);
        let value = String::from_utf8_lossy(&data[pos..pos + end]).to_string();
        let key = match field_type {
            b'S' => "severity",
            b'V' => "severity_v",
            b'C' => "code",
            b'M' => "message",
            b'D' => "detail",
            b'H' => "hint",
            b'P' => "position",
            b'W' => "where",
            _ => "unknown",
        };
        fields.insert(key.to_string(), value);
        pos += end + 1;
    }
    fields
}

fn startup(stream: &mut TcpStream) {
    const PROTOCOL_V3: i32 = 196608;
    let params = b"user\0stackql\0database\0stackql\0\0";
    let total_len = 4 + 4 + params.len();
    let mut msg = Vec::with_capacity(total_len);
    msg.extend_from_slice(&(total_len as i32).to_be_bytes());
    msg.extend_from_slice(&PROTOCOL_V3.to_be_bytes());
    msg.extend_from_slice(params);
    stream.write_all(&msg).unwrap();

    loop {
        let msg_type = read_byte(stream);
        let payload_len = read_i32(stream) as usize;
        let _data = read_bytes(stream, payload_len.saturating_sub(4));
        match msg_type {
            b'Z' => break,
            b'E' => {
                let fields = parse_error_fields(&_data);
                panic!("Startup error: {:?}", fields);
            }
            _ => {}
        }
    }
    println!("  [startup] Connected and ready");
}

fn send_query(stream: &mut TcpStream, sql: &str) {
    let sql_bytes = sql.as_bytes();
    let payload_len = 4 + sql_bytes.len() + 1;
    let mut msg = Vec::with_capacity(1 + payload_len);
    msg.push(b'Q');
    msg.extend_from_slice(&(payload_len as i32).to_be_bytes());
    msg.extend_from_slice(sql_bytes);
    msg.push(0u8);
    stream.write_all(&msg).unwrap();
}

struct QueryResponse {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    notices: Vec<String>,
    errors: Vec<HashMap<String, String>>,
    command_tag: Option<String>,
}

fn read_response(stream: &mut TcpStream) -> QueryResponse {
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut notices = Vec::new();
    let mut errors = Vec::new();
    let mut command_tag = None;

    loop {
        let msg_type = read_byte(stream);
        let payload_len = read_i32(stream) as usize;
        let data = read_bytes(stream, payload_len.saturating_sub(4));

        match msg_type {
            b'T' => {
                // RowDescription
                let num_fields = u16::from_be_bytes([data[0], data[1]]) as usize;
                let mut pos = 2;
                columns.clear();
                for _ in 0..num_fields {
                    let null_off = data[pos..].iter().position(|&b| b == 0).unwrap();
                    let name = String::from_utf8_lossy(&data[pos..pos + null_off]).to_string();
                    columns.push(name);
                    pos += null_off + 1 + 18; // skip field metadata
                }
                println!("  [T] RowDescription: {:?}", columns);
            }
            b'D' => {
                // DataRow
                let num_cols = u16::from_be_bytes([data[0], data[1]]) as usize;
                let mut pos = 2;
                let mut row = Vec::new();
                for _ in 0..num_cols {
                    let col_len = i32::from_be_bytes(data[pos..pos + 4].try_into().unwrap());
                    pos += 4;
                    if col_len < 0 {
                        row.push("NULL".to_string());
                    } else {
                        let val =
                            String::from_utf8_lossy(&data[pos..pos + col_len as usize]).to_string();
                        row.push(val);
                        pos += col_len as usize;
                    }
                }
                println!("  [D] DataRow: {:?}", row);
                rows.push(row);
            }
            b'C' => {
                // CommandComplete
                let tag =
                    String::from_utf8_lossy(data.strip_suffix(b"\0").unwrap_or(&data)).to_string();
                println!("  [C] CommandComplete: {}", tag);
                command_tag = Some(tag);
            }
            b'N' => {
                // NoticeResponse
                let fields = parse_error_fields(&data);
                let msg = fields.get("message").cloned().unwrap_or_default();
                println!("  [N] Notice: {}", msg);
                notices.push(msg);
            }
            b'E' => {
                // ErrorResponse
                let fields = parse_error_fields(&data);
                let msg = fields.get("message").cloned().unwrap_or_default();
                println!("  [E] ERROR: {}", msg);
                errors.push(fields);
            }
            b'I' => {
                println!("  [I] EmptyQueryResponse");
            }
            b'Z' => {
                let status = if data.is_empty() {
                    '?'
                } else {
                    data[0] as char
                };
                println!("  [Z] ReadyForQuery (status={})", status);
                break;
            }
            _ => {
                println!(
                    "  [{}] Unknown message ({} bytes)",
                    msg_type as char,
                    data.len()
                );
            }
        }
    }

    QueryResponse {
        columns,
        rows,
        notices,
        errors,
        command_tag,
    }
}

#[test]
#[ignore]
fn test_replace_returning_over_pgwire() {
    println!("\n=== REPLACE ... RETURNING over pgwire test ===\n");

    let mut stream = TcpStream::connect("localhost:5444")
        .expect("Failed to connect to stackql server on localhost:5444");

    startup(&mut stream);

    // Test 1: Simple SELECT to confirm connection works
    println!("\n--- Test 1: Simple SELECT ---");
    send_query(&mut stream, "SELECT 1 as test_val;");
    let resp = read_response(&mut stream);
    assert!(resp.errors.is_empty(), "Simple SELECT should not error");
    assert_eq!(resp.rows.len(), 1, "Should return 1 row");
    println!("  PASS: Simple SELECT works\n");

    // Test 2: REPLACE ... RETURNING (first attempt)
    let replace_sql = r#"REPLACE databricks_account.iam.workspace_assignment
SET
permissions = '["ADMIN"]'
WHERE
account_id = 'ebfcc5a9-9d49-4c93-b651-b3ee6cf1c9ce'
AND workspace_id = '7474653260057820'
AND principal_id = 82893155042608
RETURNING
error,
permissions,
principal;"#;

    println!("--- Test 2: REPLACE ... RETURNING (attempt 1) ---");
    send_query(&mut stream, replace_sql);
    let resp1 = read_response(&mut stream);
    println!("  Errors: {}", resp1.errors.len());
    println!("  Rows: {}", resp1.rows.len());
    println!("  Notices: {}", resp1.notices.len());
    println!("  Command tag: {:?}", resp1.command_tag);

    if !resp1.errors.is_empty() {
        println!("  ** FIRST ATTEMPT FAILED (reproduces the bug) **");
        for (i, err) in resp1.errors.iter().enumerate() {
            println!("  Error {}: {:?}", i, err);
        }
    } else {
        println!("  ** FIRST ATTEMPT SUCCEEDED **");
    }

    // Test 3: Same REPLACE ... RETURNING (second attempt)
    println!("\n--- Test 3: REPLACE ... RETURNING (attempt 2) ---");
    send_query(&mut stream, replace_sql);
    let resp2 = read_response(&mut stream);
    println!("  Errors: {}", resp2.errors.len());
    println!("  Rows: {}", resp2.rows.len());
    println!("  Notices: {}", resp2.notices.len());
    println!("  Command tag: {:?}", resp2.command_tag);

    if !resp2.errors.is_empty() {
        println!("  ** SECOND ATTEMPT ALSO FAILED **");
        for (i, err) in resp2.errors.iter().enumerate() {
            println!("  Error {}: {:?}", i, err);
        }
    } else {
        println!("  ** SECOND ATTEMPT SUCCEEDED **");
    }

    // Test 4: Simple INSERT ... RETURNING on a CC resource for comparison
    println!("\n--- Test 4: Simple SELECT for sanity ---");
    send_query(&mut stream, "SELECT 1 as still_alive;");
    let resp3 = read_response(&mut stream);
    assert!(resp3.errors.is_empty(), "Connection should still be alive");
    println!("  PASS: Connection still alive after REPLACE tests\n");

    // Summary
    println!("=== SUMMARY ===");
    println!(
        "  Attempt 1: {}",
        if resp1.errors.is_empty() {
            "SUCCESS"
        } else {
            "FAILED"
        }
    );
    println!(
        "  Attempt 2: {}",
        if resp2.errors.is_empty() {
            "SUCCESS"
        } else {
            "FAILED"
        }
    );
    if !resp1.errors.is_empty() && resp2.errors.is_empty() {
        println!("  CONCLUSION: Bug reproduced - first attempt fails, second succeeds");
    } else if resp1.errors.is_empty() && resp2.errors.is_empty() {
        println!("  CONCLUSION: Both succeeded - bug not reproduced this time");
    } else {
        println!("  CONCLUSION: Unexpected pattern");
    }
}
