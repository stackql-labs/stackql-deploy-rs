// utils/pgwire.rs

//! Pure-Rust PostgreSQL simple-query wire protocol client.
//!
//! Implements only what stackql-deploy needs: unencrypted TCP connections
//! to a local StackQL server using the PostgreSQL simple query protocol (v3).
//! No native dependencies (replaces pgwire-lite → libpq-sys).

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::TcpStream;

/// A single column value returned from a query.
pub enum Value {
    String(String),
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    Bytes(Vec<u8>),
}

/// A server notice (NOTICE, WARNING, etc.).
pub struct Notice {
    pub fields: HashMap<String, String>,
}

/// The result of a [`PgwireLite::query`] call.
pub struct PgQueryResult {
    pub column_names: Vec<String>,
    pub rows: Vec<HashMap<String, Value>>,
    pub notices: Vec<Notice>,
    /// Row count reported by CommandComplete (INSERT/UPDATE/DELETE n).
    pub row_count: usize,
}

/// Minimal PostgreSQL wire-protocol client.
pub struct PgwireLite {
    stream: TcpStream,
    /// Canonical signatures of every notice line surfaced earlier in this
    /// session. stackql emits each new query's NoticeResponse with a
    /// cumulative `detail` field containing every provider notice seen so
    /// far. Any detail line already present in this set is stale and
    /// dropped from subsequent query results.
    seen_notice_sigs: HashSet<String>,
}

impl PgwireLite {
    /// Connect to a PostgreSQL-protocol server (e.g. StackQL) at `host:port`.
    ///
    /// `_ssl` and `_verbosity` are accepted for API compatibility but ignored;
    /// the connection is always unencrypted (StackQL default).
    pub fn new(host: &str, port: u16, _ssl: bool, _verbosity: &str) -> Result<Self, String> {
        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect(&addr)
            .map_err(|e| format!("Connection to {} failed: {}", addr, e))?;

        let mut client = PgwireLite {
            stream,
            seen_notice_sigs: HashSet::new(),
        };
        client.startup()?;
        Ok(client)
    }

    /// Returns a version string (no libpq; just identifies the client).
    pub fn libpq_version(&self) -> String {
        "pure-rust-pgwire-client".to_string()
    }

    // ------------------------------------------------------------------
    // Startup handshake
    // ------------------------------------------------------------------

    fn startup(&mut self) -> Result<(), String> {
        // Protocol version 3.0 = 0x00_03_00_00
        const PROTOCOL_V3: i32 = 196608;

        // Startup message: user=stackql, database=stackql, then double-null
        let params = b"user\0stackql\0database\0stackql\0\0";
        let total_len = 4 + 4 + params.len(); // length field + protocol + params

        let mut msg = Vec::with_capacity(total_len);
        msg.extend_from_slice(&(total_len as i32).to_be_bytes());
        msg.extend_from_slice(&PROTOCOL_V3.to_be_bytes());
        msg.extend_from_slice(params);

        self.stream
            .write_all(&msg)
            .map_err(|e| format!("Startup write error: {}", e))?;

        // Process auth / parameter-status messages until ReadyForQuery
        loop {
            let msg_type = self.read_byte()?;
            let payload_len = self.read_i32()? as usize;
            // payload_len includes the 4 bytes of the length field itself
            let data = self.read_bytes(payload_len.saturating_sub(4))?;

            match msg_type {
                b'R' => {
                    // AuthenticationRequest
                    let auth_type =
                        i32::from_be_bytes(data[..4].try_into().map_err(|_| "Bad auth")?);
                    if auth_type != 0 {
                        return Err(format!(
                            "Unsupported authentication type {} from server",
                            auth_type
                        ));
                    }
                    // AuthenticationOk — nothing to do
                }
                b'K' => {}     // BackendKeyData — ignore
                b'S' => {}     // ParameterStatus — ignore
                b'Z' => break, // ReadyForQuery
                b'E' => return Err(parse_error_fields(&data)),
                b'N' => {} // NoticeResponse during startup — ignore
                _ => {}    // Unknown message type — skip
            }
        }

        Ok(())
    }

    // ------------------------------------------------------------------
    // Query
    // ------------------------------------------------------------------

    /// Execute a simple (non-prepared) SQL query and return structured results.
    pub fn query(&mut self, sql: &str) -> Result<PgQueryResult, String> {
        // Drain any bytes the server may have emitted outside a prior query's
        // response window. stackql has been observed to re-emit stale
        // NoticeResponse frames from earlier statements, which would
        // otherwise be attributed to this query.
        self.drain_pending();

        // Send Query message: 'Q' | int32(len) | sql\0
        let sql_bytes = sql.as_bytes();
        let payload_len = 4 + sql_bytes.len() + 1; // length field + sql + null

        let mut msg = Vec::with_capacity(1 + payload_len);
        msg.push(b'Q');
        msg.extend_from_slice(&(payload_len as i32).to_be_bytes());
        msg.extend_from_slice(sql_bytes);
        msg.push(0u8);

        self.stream
            .write_all(&msg)
            .map_err(|e| format!("Query write error: {}", e))?;

        // Collect response messages
        let mut column_names: Vec<String> = Vec::new();
        let mut rows: Vec<HashMap<String, Value>> = Vec::new();
        let mut notices: Vec<Notice> = Vec::new();
        let mut row_count: usize = 0;

        loop {
            let msg_type = self.read_byte()?;
            let payload_len = self.read_i32()? as usize;
            let data = self.read_bytes(payload_len.saturating_sub(4))?;

            match msg_type {
                b'T' => {
                    // RowDescription
                    column_names = parse_row_description(&data);
                }
                b'D' => {
                    // DataRow
                    let row = parse_data_row(&data, &column_names);
                    rows.push(row);
                }
                b'C' => {
                    // CommandComplete — tag like "SELECT 5", "INSERT 0 1", "UPDATE 3"
                    let tag = std::str::from_utf8(data.strip_suffix(b"\0").unwrap_or(&data))
                        .unwrap_or("")
                        .to_string();
                    if let Some(n) = tag.split_whitespace().last().and_then(|s| s.parse().ok()) {
                        row_count = n;
                    }
                }
                b'N' => {
                    notices.push(parse_notice_fields(&data));
                }
                b'E' => {
                    // Capture the error but DON'T return yet — we must
                    // drain the stream until ReadyForQuery ('Z') so the
                    // connection is left in a clean state for the next query.
                    let err_msg = parse_error_fields(&data);
                    // Continue reading until ReadyForQuery
                    loop {
                        let drain_type = self.read_byte()?;
                        let drain_len = self.read_i32()? as usize;
                        let _drain_data = self.read_bytes(drain_len.saturating_sub(4))?;
                        if drain_type == b'Z' {
                            break;
                        }
                    }
                    return Err(err_msg);
                }
                b'I' => {}     // EmptyQueryResponse
                b'Z' => break, // ReadyForQuery — done
                _ => {}
            }
        }

        // Drop any detail lines we've already surfaced earlier in this
        // session; stackql keeps re-emitting them on every subsequent
        // query's NoticeResponse.
        let kept = filter_stale_notices(notices, &mut self.seen_notice_sigs);

        Ok(PgQueryResult {
            column_names,
            rows,
            notices: kept,
            row_count,
        })
    }

    /// Discard any bytes the server sent outside a query response window.
    ///
    /// The server is expected to stay silent between queries (the prior
    /// response ended with `ReadyForQuery` / `'Z'`). stackql has been
    /// observed to emit stray `NoticeResponse` frames referring to
    /// previously-executed statements; if left in the socket buffer those
    /// frames would be read at the start of the next query and
    /// misattributed. Best-effort non-blocking read; any I/O error is
    /// swallowed since the normal path is zero bytes available.
    fn drain_pending(&mut self) {
        if self.stream.set_nonblocking(true).is_err() {
            return;
        }
        let mut buf = [0u8; 4096];
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => break,    // EOF
                Ok(_) => continue, // keep draining
                Err(_) => break,   // WouldBlock or other — nothing more pending
            }
        }
        let _ = self.stream.set_nonblocking(false);
    }

    // ------------------------------------------------------------------
    // Low-level I/O helpers
    // ------------------------------------------------------------------

    fn read_byte(&mut self) -> Result<u8, String> {
        let mut buf = [0u8; 1];
        self.stream
            .read_exact(&mut buf)
            .map_err(|e| format!("Read error: {}", e))?;
        Ok(buf[0])
    }

    fn read_i32(&mut self) -> Result<i32, String> {
        let mut buf = [0u8; 4];
        self.stream
            .read_exact(&mut buf)
            .map_err(|e| format!("Read error: {}", e))?;
        Ok(i32::from_be_bytes(buf))
    }

    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>, String> {
        let mut buf = vec![0u8; n];
        self.stream
            .read_exact(&mut buf)
            .map_err(|e| format!("Read error: {}", e))?;
        Ok(buf)
    }
}

// ------------------------------------------------------------------
// Stale notice filtering
// ------------------------------------------------------------------

/// Build a signature for a notice line. Dedup uses exact byte-match (after
/// trimming) so two genuine provider errors — which always differ in their
/// embedded request/serving IDs — are never conflated. A re-emitted stale
/// notice is byte-identical to its first emission and dedups cleanly.
fn canonical_line(line: &str) -> String {
    line.trim().to_string()
}

/// Drop detail lines whose canonical signature was already surfaced earlier
/// in the session. `seen` is mutated in place: every line that survives
/// filtering is added so it counts as stale on subsequent queries. A notice
/// whose message is stale and whose detail becomes empty after filtering is
/// dropped entirely.
fn filter_stale_notices(notices: Vec<Notice>, seen: &mut HashSet<String>) -> Vec<Notice> {
    let mut kept = Vec::with_capacity(notices.len());

    for n in notices {
        let message_stale = n
            .fields
            .get("message")
            .map(|m| seen.contains(&format!("M|{}", canonical_line(m))))
            .unwrap_or(false);

        let filtered_detail = n.fields.get("detail").map(|detail| {
            detail
                .lines()
                .filter(|line| {
                    let c = canonical_line(line);
                    c.is_empty() || !seen.contains(&format!("D|{}", c))
                })
                .collect::<Vec<_>>()
                .join("\n")
        });

        let has_detail_content = filtered_detail
            .as_deref()
            .map(|d| !d.trim().is_empty())
            .unwrap_or(false);

        if message_stale && !has_detail_content {
            continue;
        }

        // Mark the surviving signatures as seen so they're filtered from
        // future queries.
        if let Some(m) = n.fields.get("message") {
            let c = canonical_line(m);
            if !c.is_empty() {
                seen.insert(format!("M|{}", c));
            }
        }
        if let Some(d) = filtered_detail.as_deref() {
            for line in d.lines() {
                let c = canonical_line(line);
                if !c.is_empty() {
                    seen.insert(format!("D|{}", c));
                }
            }
        }

        let mut new_fields = n.fields.clone();
        if let Some(d) = filtered_detail {
            if d.trim().is_empty() {
                new_fields.remove("detail");
            } else {
                new_fields.insert("detail".to_string(), d);
            }
        }
        kept.push(Notice { fields: new_fields });
    }

    kept
}

// ------------------------------------------------------------------
// Message parsers (free functions for readability)
// ------------------------------------------------------------------

fn parse_row_description(data: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    if data.len() < 2 {
        return names;
    }
    let num_fields = u16::from_be_bytes([data[0], data[1]]) as usize;
    let mut pos = 2;

    for _ in 0..num_fields {
        // Null-terminated field name
        let Some(null_off) = data[pos..].iter().position(|&b| b == 0) else {
            break;
        };
        let name = String::from_utf8_lossy(&data[pos..pos + null_off]).into_owned();
        names.push(name);
        // Skip: name + null(1) + tableOID(4) + attrNum(2) + typeOID(4) + typeSize(2)
        //       + typeMod(4) + formatCode(2) = 19 bytes after the null
        pos += null_off + 1 + 18;
    }
    names
}

fn parse_data_row(data: &[u8], columns: &[String]) -> HashMap<String, Value> {
    let mut row = HashMap::new();
    if data.len() < 2 {
        return row;
    }
    let num_cols = u16::from_be_bytes([data[0], data[1]]) as usize;
    let mut pos = 2;

    for col_name in columns.iter().take(num_cols) {
        if pos + 4 > data.len() {
            break;
        }
        let col_len = i32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos += 4;

        let value = if col_len < 0 {
            Value::Null
        } else {
            let len = col_len as usize;
            if pos + len > data.len() {
                break;
            }
            let s = String::from_utf8_lossy(&data[pos..pos + len]).into_owned();
            pos += len;
            Value::String(s)
        };

        row.insert(col_name.clone(), value);
    }
    row
}

fn parse_notice_fields(data: &[u8]) -> Notice {
    let mut fields = HashMap::new();
    let mut pos = 0;

    while pos < data.len() {
        let field_code = data[pos];
        pos += 1;
        if field_code == 0 {
            break;
        }
        let Some(null_off) = data[pos..].iter().position(|&b| b == 0) else {
            break;
        };
        let value = String::from_utf8_lossy(&data[pos..pos + null_off]).into_owned();
        pos += null_off + 1;

        let key = match field_code {
            b'S' => "severity",
            b'M' => "message",
            b'D' => "detail",
            b'H' => "hint",
            b'C' => "code",
            b'P' => "position",
            b'W' => "where",
            _ => continue,
        };
        fields.insert(key.to_string(), value);
    }

    Notice { fields }
}

fn parse_error_fields(data: &[u8]) -> String {
    let mut pos = 0;
    while pos < data.len() {
        let field_code = data[pos];
        pos += 1;
        if field_code == 0 {
            break;
        }
        let Some(null_off) = data[pos..].iter().position(|&b| b == 0) else {
            break;
        };
        let value = String::from_utf8_lossy(&data[pos..pos + null_off]).into_owned();
        pos += null_off + 1;
        if field_code == b'M' {
            return value;
        }
    }
    "Unknown server error".to_string()
}
