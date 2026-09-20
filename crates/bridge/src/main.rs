use flashtex_bridge::{
    provider::{ProviderConfig, ProviderKind},
    store::Store,
    *,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};

const MAX_FRAME: usize = 12 * 1024 * 1024;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    protocol_version: u8,
    id: String,
    #[serde(rename = "type")]
    kind: String,
    payload: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    destination_id: String,
    project_id: String,
    path: String,
    revision: u64,
    start_byte: usize,
    end_byte: usize,
    /// Additive: `fixed` (default, the explicit pin) or `caret` (the Mac's
    /// automatic destination; see `AnchorMode`).
    #[serde(default)]
    mode: AnchorMode,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Convert {
    capture_id: String,
    // Accepted for wire compatibility but intentionally unused: the honest
    // supported-feature list is derived from the compiler's own tables
    // (`features::supported_features`) rather than trusted from the caller,
    // so it cannot drift into a hand-maintained overstatement (issues #51/#23).
    #[serde(default)]
    #[allow(dead_code)]
    supported_features: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Prepare {
    capture_id: String,
    expected_revision: u64,
    approved: bool,
    /// Additive: text the Mac puts around the proposal at approval
    /// (`InsertionWrap`); journaled with the prepared edit.
    #[serde(default)]
    wrap: Option<InsertionWrap>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Confirm {
    capture_id: String,
    edit_id: String,
    new_revision: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureId {
    capture_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ValidateCapture {
    capture_id: String,
    expected_revision: u64,
}
fn decode<T: serde::de::DeserializeOwned>(payload: Value) -> Result<T> {
    Ok(serde_json::from_value(payload)?)
}
fn dispatch(
    bridge: &mut Bridge,
    message: Envelope,
    provider: Option<ProviderKind>,
    compiler: Option<&(validation::CompilerValidator, String)>,
) -> Result<(&'static str, Value)> {
    if message.protocol_version != 1 {
        return Err(BridgeError::new(
            "unsupported_version",
            "Only protocol_version 1 is supported",
        ));
    }
    match message.kind.as_str() {
        "capture_validate" => {
            let request: ValidateCapture = decode(message.payload)?;
            let (compiler, entry) = compiler.ok_or_else(|| {
                BridgeError::new(
                    "compiler_not_configured",
                    "Configure the original FlashTeX compiler and project entry before validation",
                )
            })?;
            let evidence = bridge.validate_capture(
                &message.id,
                &request.capture_id,
                request.expected_revision,
                entry,
                compiler,
            )?;
            Ok((
                "capture_validation",
                json!({"capture_id":request.capture_id,"context_revision":request.expected_revision,"validation":evidence}),
            ))
        }
        "document_open" => {
            let doc: Document = decode(message.payload)?;
            bridge.open_document(doc)?;
            Ok(("document_opened", json!({})))
        }
        "document_edit" => {
            let edit: EditRequest = decode(message.payload)?;
            bridge.edit(&edit)?;
            Ok(("document_updated", json!({"revision":edit.revision})))
        }
        "destination_pin" => {
            let pin: Pin = decode(message.payload)?;
            let a = bridge.pin_with_mode(
                &pin.destination_id,
                &pin.project_id,
                &pin.path,
                pin.revision,
                pin.start_byte,
                pin.end_byte,
                pin.mode,
            )?;
            Ok(("destination_pinned", serde_json::to_value(a)?))
        }
        "capture_submit" => {
            let capture: CaptureSubmit = decode(message.payload)?;
            let record = bridge.receive(capture)?;
            Ok((
                "capture_received",
                json!({"capture_id":record.capture.capture_id,"durable":true,"has_proposal":record.proposal.is_some(),"applied":record.applied.is_some()}),
            ))
        }
        "capture_convert" => {
            let request: Convert = decode(message.payload)?;
            let kind = provider.ok_or_else(|| {
                BridgeError::new(
                    "provider_disabled",
                    "Capture conversion requires an explicitly enabled provider (--conversion-provider)",
                )
            })?;
            // Resolved per request so a missing key is a request error, not a
            // startup failure; building the converter sends nothing.
            let converter =
                ProviderConfig::resolve(kind, |name| std::env::var(name).ok())?.build()?;
            let record = bridge.convert(
                &request.capture_id,
                features::supported_features(),
                converter.as_ref(),
            )?;
            let proposal = record.proposal.unwrap();
            let insertion_blocked = proposal.blocks_direct_insertion();
            Ok((
                "capture_proposal",
                json!({"capture_id":request.capture_id,"latex":proposal.latex,"ambiguities":proposal.ambiguities,"required_dependencies":proposal.required_dependencies,"context_revision":record.context.map(|c|c.revision),"insertion_blocked":insertion_blocked,"provider_evidence":record.provider_evidence}),
            ))
        }
        "capture_prepare_insert" => {
            let request: Prepare = decode(message.payload)?;
            let edit = bridge.prepare_insert_wrapped(
                &request.capture_id,
                request.expected_revision,
                request.approved,
                request.wrap,
            )?;
            Ok(("capture_edit", serde_json::to_value(edit)?))
        }
        "capture_applied" => {
            let request: Confirm = decode(message.payload)?;
            let receipt = bridge.confirm_insert(
                &request.capture_id,
                &request.edit_id,
                request.new_revision,
            )?;
            // Additive: the wrap the applied edit was prepared with, so a
            // receipt says what was put around the proposal.
            let wrap = bridge
                .store
                .get(&request.capture_id)?
                .and_then(|r| r.prepared)
                .and_then(|e| e.wrap);
            Ok((
                "capture_application_received",
                json!({"capture_id":request.capture_id,"edit_id":receipt.edit_id,"new_revision":receipt.new_revision,"wrap":wrap}),
            ))
        }
        "capture_status" => {
            let request: CaptureId = decode(message.payload)?;
            let record = bridge.store.require(&request.capture_id)?;
            Ok((
                "capture_status",
                json!({"capture_id":request.capture_id,"proposal":record.proposal,"prepared":record.prepared,"applied":record.applied,"rejected":record.rejected,"provider_evidence":record.provider_evidence}),
            ))
        }
        "capture_reject" => {
            let request: CaptureId = decode(message.payload)?;
            bridge.reject(&request.capture_id)?;
            Ok(("capture_rejected", json!({"capture_id":request.capture_id})))
        }
        _ => Err(BridgeError::new(
            "unsupported_type",
            "Unknown bridge request type",
        )),
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut store = None;
    // `None` = not given; `Some(None)` = explicitly `none`.
    let mut provider: Option<Option<ProviderKind>> = None;
    let mut compiler_path = None;
    let mut compiler_entry = None;
    let choose = |current: &mut Option<Option<ProviderKind>>, kind: Option<ProviderKind>| {
        if current.is_some_and(|existing| existing != kind) {
            return Err(BridgeError::new(
                "invalid_arguments",
                "--enable-grok and --conversion-provider disagree; pass one provider",
            ));
        }
        *current = Some(kind);
        Ok(())
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--store" => store = args.next(),
            "--conversion-provider" => {
                let name = args.next().ok_or_else(|| {
                    BridgeError::new(
                        "invalid_arguments",
                        "--conversion-provider needs none, xai or openai-compatible",
                    )
                })?;
                choose(&mut provider, ProviderKind::parse(&name)?)?;
            }
            // Deprecated alias for `--conversion-provider xai`; kept one release.
            "--enable-grok" => choose(&mut provider, Some(ProviderKind::Xai))?,
            "--compiler" => compiler_path = args.next(),
            "--compiler-entry" => compiler_entry = args.next(),
            "--help" => {
                println!("flashtex-bridge --store PRIVATE_APP_DATA_DIRECTORY [--conversion-provider none|xai|openai-compatible] [--compiler ORIGINAL_FLASHTEX_BINARY --compiler-entry main.tex]\nReads runtime-v1 JSONLines from stdin; logs to stderr. No network calls without capture_convert and a --conversion-provider other than none (--enable-grok is a deprecated alias for xai).\nProvider environment: FLASHTEX_AI_API_KEY (xai also XAI_API_KEY), FLASHTEX_CONVERSION_MODEL (xai also FLASHTEX_GROK_MODEL), FLASHTEX_CONVERSION_BASE_URL (https, or http on loopback).");
                return Ok(());
            }
            _ => {
                return Err(BridgeError::new(
                    "invalid_arguments",
                    "Use --store DIRECTORY and optional --conversion-provider NAME",
                ))
            }
        }
    }
    let provider = provider.flatten();
    let compiler = match (compiler_path, compiler_entry) {
        (None, None) => None,
        (Some(path), Some(entry)) => {
            relative_path(&entry)?;
            Some((
                validation::CompilerValidator {
                    executable: path.into(),
                    timeout: std::time::Duration::from_secs(5),
                },
                entry,
            ))
        }
        _ => {
            return Err(BridgeError::new(
                "invalid_arguments",
                "Provide both --compiler and --compiler-entry",
            ))
        }
    };
    let mut bridge = Bridge::new(Store::open(store.ok_or_else(|| {
        BridgeError::new(
            "invalid_arguments",
            "A private capture journal directory is required",
        )
    })?)?);
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    loop {
        let mut frame = Vec::new();
        let count = reader
            .by_ref()
            .take((MAX_FRAME + 1) as u64)
            .read_until(b'\n', &mut frame)?;
        if count == 0 {
            break;
        }
        let mut id = Value::Null;
        let result = if frame.len() > MAX_FRAME {
            if frame.last() != Some(&b'\n') {
                loop {
                    let buf = reader.fill_buf()?;
                    if buf.is_empty() {
                        break;
                    }
                    let n = buf
                        .iter()
                        .position(|b| *b == b'\n')
                        .map(|p| p + 1)
                        .unwrap_or(buf.len());
                    let ended = buf[n - 1] == b'\n';
                    reader.consume(n);
                    if ended {
                        break;
                    }
                }
            }
            Err(BridgeError::new(
                "message_too_large",
                "Bridge message exceeded 12 MiB; record discarded",
            ))
        } else {
            let value: std::result::Result<Value, _> = serde_json::from_slice(&frame);
            match value {
                Ok(value) => {
                    id = value
                        .get("id")
                        .filter(|v| v.as_str().is_some_and(|s| !s.is_empty() && s.len() <= 128))
                        .cloned()
                        .unwrap_or(Value::Null);
                    if id.is_null() {
                        Err(BridgeError::new(
                            "invalid_id",
                            "Every request needs a bounded nonempty string ID",
                        ))
                    } else {
                        decode(value).and_then(|message: Envelope| {
                            let _ = &message.id;
                            dispatch(&mut bridge, message, provider, compiler.as_ref())
                        })
                    }
                }
                Err(_) => Err(BridgeError::new(
                    "invalid_json",
                    "Malformed UTF-8 JSON request",
                )),
            }
        };
        let reply = match result {
            Ok((kind, payload)) => {
                json!({"protocol_version":1,"id":id,"type":kind,"payload":payload})
            }
            Err(error) => json!({"protocol_version":1,"id":id,"type":"error","payload":error}),
        };
        serde_json::to_writer(&mut writer, &reply)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }
    Ok(())
}
