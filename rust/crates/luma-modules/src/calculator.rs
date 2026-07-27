use crate::cancel::await_unless_cancelled;
use async_trait::async_trait;
use chrono::{DateTime, Days, NaiveDate, Utc};
use luma_application::{
    ActionOutcome, ActionRequest, LumaModule, ModuleManifest, ModuleState, PasteboardPort,
    SearchMode, SearchSink, WarmupContext,
};
use luma_domain::{
    ActionDescriptor, ActionId, ActionRisk, FailureKind, ModuleId, Query, QueryScope, SearchItem,
};
use luma_protocol::{ActionDescriptorDto, Event, SearchItemDto};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

const MODULE_ID: &str = "luma.calculator";
const MAX_INPUT_CHARS: usize = 256;
const MAX_TOKENS: usize = 128;
const MAX_NESTING: usize = 32;
const MAX_POWER_MAGNITUDE: f64 = 1_024.0;
const MAX_OUTPUT_CHARS: usize = 1_024;

pub struct CalculatorModule {
    manifest: ModuleManifest,
    pasteboard: Arc<dyn PasteboardPort>,
}

impl CalculatorModule {
    pub fn with_deps(pasteboard: Arc<dyn PasteboardPort>) -> Self {
        Self {
            manifest: ModuleManifest {
                id: ModuleId::new(MODULE_ID),
                display_name: "Calculator".into(),
                triggers: vec!["calc".into(), "calculate".into()],
                default_enabled: true,
                search_mode: SearchMode::GlobalContributing,
                required_capabilities: vec![],
                workbench: luma_application::WorkbenchMeta {
                    glyph: Some("=".into()),
                    suggested_query: Some("/calc ".into()),
                    empty_hint: Some("/calc 1 + 2 · unit conversion · base/date helpers".into()),
                    supports_browse: false,
                    commands: vec![crate::ux::command_spec(
                        "/calc <expression>",
                        "Calculate arithmetic, units, bases, Unix time, or date offsets",
                        "/calc ",
                        Some("/calc 128 MiB in GiB"),
                    )],
                },
            },
            pasteboard,
        }
    }
}

#[async_trait]
impl LumaModule for CalculatorModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn warmup(&self, _ctx: WarmupContext) -> ModuleState {
        ModuleState::Ready
    }

    async fn search(&self, query: Query, sink: SearchSink, cancel: CancellationToken) {
        if cancel.is_cancelled() {
            return;
        }
        let targeted = matches!(query.scope, QueryScope::Targeted { .. });
        let input = if targeted {
            query.rest_raw()
        } else {
            query.raw.trim()
        };

        if input.is_empty() {
            if targeted {
                send_one(
                    &sink,
                    SearchItemDto {
                        id: "calc:help".into(),
                        module_id: MODULE_ID.into(),
                        title: "Enter a calculation".into(),
                        subtitle: Some(
                            "Arithmetic, unit conversion, /calc base, /calc unix, or /calc date"
                                .into(),
                        ),
                        kind: "onboarding".into(),
                        score: 0.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "Ready".into(),
                        ..Default::default()
                    },
                )
                .await;
            }
            return;
        }

        if !targeted && !is_strict_global_expression(input) {
            return;
        }

        let calculation = calculate(input);
        if cancel.is_cancelled() {
            return;
        }
        match calculation {
            Ok(calculation) => send_one(&sink, calculation_item(input, calculation)).await,
            Err(error) if targeted => {
                send_one(
                    &sink,
                    SearchItemDto {
                        id: format!("calc:error:{:016x}", stable_hash(input)),
                        module_id: MODULE_ID.into(),
                        title: "Calculation failed".into(),
                        subtitle: Some(error.to_string()),
                        kind: "command_error".into(),
                        score: 0.0,
                        primary_action_id: "noop".into(),
                        primary_action_label: "Fix input".into(),
                        ..Default::default()
                    },
                )
                .await;
            }
            Err(_) => {}
        }
    }

    async fn actions(&self, result: &SearchItem) -> Vec<ActionDescriptor> {
        if result.kind != "calculation" {
            return vec![safe_action("noop", "OK")];
        }
        let mut actions = vec![
            safe_action("copy_result", "Copy result"),
            safe_action("copy_equation", "Copy equation"),
        ];
        if let Some(integer) = payload_integer(result) {
            if integer >= 0 {
                actions.push(safe_action("copy_decimal", "Copy decimal"));
                actions.push(safe_action("copy_hex", "Copy hex"));
            }
        }
        actions
    }

    async fn perform(&self, request: ActionRequest, cancel: CancellationToken) -> ActionOutcome {
        if cancel.is_cancelled() {
            return ActionOutcome::Cancelled;
        }
        if request.action.id.as_str() == "noop" {
            return ActionOutcome::Success { message: None };
        }
        let Some(payload) = request.result.action_payload.as_ref() else {
            return invalid_payload("missing calculator action payload");
        };
        let text = match request.action.id.as_str() {
            "copy_result" => payload
                .get("result")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            "copy_equation" => payload
                .get("equation")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            "copy_decimal" => payload_integer(&request.result)
                .filter(|value| *value >= 0)
                .map(|value| value.to_string()),
            "copy_hex" => payload_integer(&request.result)
                .filter(|value| *value >= 0)
                .map(|value| format!("0x{value:X}")),
            _ => return invalid_payload("unknown calculator action"),
        };
        let Some(text) = text else {
            return invalid_payload("calculator action is not available for this result");
        };
        match await_unless_cancelled(&cancel, self.pasteboard.write_text(&text)).await {
            None => ActionOutcome::Cancelled,
            Some(Ok(())) => ActionOutcome::Success {
                message: Some("copied calculation".into()),
            },
            Some(Err(error)) => ActionOutcome::Failed {
                kind: FailureKind::Unavailable {
                    reason: error.to_string(),
                    retryable: true,
                },
            },
        }
    }

    async fn teardown(&self) {}
}

fn invalid_payload(message: &str) -> ActionOutcome {
    ActionOutcome::Failed {
        kind: FailureKind::InvalidInput {
            field: "action_payload".into(),
            message: message.into(),
        },
    }
}

fn payload_integer(result: &SearchItem) -> Option<i64> {
    result.action_payload.as_ref()?.get("integer")?.as_i64()
}

fn safe_action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor {
        id: ActionId::new(id),
        label: label.into(),
        risk: ActionRisk::Safe,
        confirmation: false,
    }
}

async fn send_one(sink: &SearchSink, item: SearchItemDto) {
    let _ = sink
        .send(Event::ResultsChunk {
            request_id: String::new(),
            sequence: 1,
            upserts: vec![item],
            removed_ids: vec![],
        })
        .await;
}

#[derive(Clone, Debug, PartialEq)]
struct Calculation {
    result: String,
    integer: Option<i64>,
}

fn calculation_item(input: &str, calculation: Calculation) -> SearchItemDto {
    let equation = format!("{input} = {}", calculation.result);
    let mut secondary_actions = vec![ActionDescriptorDto {
        id: "copy_equation".into(),
        label: "Copy equation".into(),
        risk: ActionRisk::Safe,
        confirmation: false,
    }];
    if calculation.integer.is_some_and(|value| value >= 0) {
        secondary_actions.extend([
            ActionDescriptorDto {
                id: "copy_decimal".into(),
                label: "Copy decimal".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
            ActionDescriptorDto {
                id: "copy_hex".into(),
                label: "Copy hex".into(),
                risk: ActionRisk::Safe,
                confirmation: false,
            },
        ]);
    }
    SearchItemDto {
        id: format!("calc:{:016x}", stable_hash(input)),
        module_id: MODULE_ID.into(),
        title: calculation.result.clone(),
        subtitle: Some(equation.clone()),
        kind: "calculation".into(),
        score: 100.0,
        primary_action_id: "copy_result".into(),
        primary_action_label: "Copy result".into(),
        secondary_actions,
        action_payload: Some(serde_json::json!({
            "result": calculation.result,
            "equation": equation,
            "integer": calculation.integer,
        })),
        ..Default::default()
    }
}

fn stable_hash(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CalcError {
    Invalid(String),
    Limit(String),
}

impl std::fmt::Display for CalcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) | Self::Limit(message) => f.write_str(message),
        }
    }
}

fn calculate(input: &str) -> Result<Calculation, CalcError> {
    if input.chars().count() > MAX_INPUT_CHARS {
        return Err(CalcError::Limit(format!(
            "input exceeds {MAX_INPUT_CHARS} characters"
        )));
    }
    let trimmed = input.trim();
    let calculation = if let Some(rest) = trimmed.strip_prefix("base ") {
        calculate_base(rest)?
    } else if let Some(rest) = trimmed.strip_prefix("unix ") {
        calculate_unix(rest)?
    } else if let Some(rest) = trimmed.strip_prefix("date ") {
        calculate_date(rest)?
    } else if looks_like_conversion(trimmed) {
        calculate_conversion(trimmed)?
    } else {
        let value = Parser::new(trimmed)?.parse()?;
        let result = format_number(value)?;
        Calculation {
            integer: exact_i64(value),
            result,
        }
    };
    if calculation.result.chars().count() > MAX_OUTPUT_CHARS {
        return Err(CalcError::Limit(format!(
            "output exceeds {MAX_OUTPUT_CHARS} characters"
        )));
    }
    Ok(calculation)
}

fn is_strict_global_expression(input: &str) -> bool {
    if input.is_empty() || input.chars().count() > MAX_INPUT_CHARS {
        return false;
    }
    if looks_like_conversion(input) {
        return calculate_conversion(input).is_ok();
    }
    if looks_like_iso_date(input) {
        return false;
    }
    if input
        .chars()
        .any(|ch| !(ch.is_ascii_digit() || " \t._+-*/^%()eE".contains(ch)))
    {
        return false;
    }
    let has_operation = input.char_indices().any(|(index, ch)| {
        matches!(ch, '*' | '/' | '^' | '%' | '(' | ')') || (matches!(ch, '+' | '-') && index > 0)
    });
    has_operation && Parser::new(input).and_then(Parser::parse).is_ok()
}

fn looks_like_iso_date(input: &str) -> bool {
    let bytes = input.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn looks_like_conversion(input: &str) -> bool {
    input.split_ascii_whitespace().count() == 4
        && input
            .split_ascii_whitespace()
            .nth(2)
            .is_some_and(|word| word.eq_ignore_ascii_case("in"))
}

fn calculate_conversion(input: &str) -> Result<Calculation, CalcError> {
    let fields = input.split_ascii_whitespace().collect::<Vec<_>>();
    if fields.len() != 4 || !fields[2].eq_ignore_ascii_case("in") {
        return Err(CalcError::Invalid(
            "expected: <value> <unit> in <unit>".into(),
        ));
    }
    let value = parse_decimal(fields[0])?;
    let from = Unit::parse(fields[1])
        .ok_or_else(|| CalcError::Invalid(format!("unsupported unit: {}", fields[1])))?;
    let to = Unit::parse(fields[3])
        .ok_or_else(|| CalcError::Invalid(format!("unsupported unit: {}", fields[3])))?;
    if from.dimension() != to.dimension() {
        return Err(CalcError::Invalid("incompatible unit dimensions".into()));
    }
    let converted = convert_units(value, from, to)?;
    Ok(Calculation {
        result: format!("{} {}", format_number(converted)?, to.symbol()),
        integer: None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dimension {
    Data,
    Temperature,
    Duration,
    Length,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Unit {
    Linear {
        symbol: &'static str,
        dimension: Dimension,
        factor: f64,
    },
    Celsius,
    Fahrenheit,
    Kelvin,
}

impl Unit {
    fn parse(raw: &str) -> Option<Self> {
        let linear = |symbol, dimension, factor| Self::Linear {
            symbol,
            dimension,
            factor,
        };
        Some(match raw {
            "B" => linear("B", Dimension::Data, 1.0),
            "KB" => linear("KB", Dimension::Data, 1_000.0),
            "MB" => linear("MB", Dimension::Data, 1_000_000.0),
            "GB" => linear("GB", Dimension::Data, 1_000_000_000.0),
            "TB" => linear("TB", Dimension::Data, 1_000_000_000_000.0),
            "KiB" => linear("KiB", Dimension::Data, 1_024.0),
            "MiB" => linear("MiB", Dimension::Data, 1_048_576.0),
            "GiB" => linear("GiB", Dimension::Data, 1_073_741_824.0),
            "TiB" => linear("TiB", Dimension::Data, 1_099_511_627_776.0),
            "C" => Self::Celsius,
            "F" => Self::Fahrenheit,
            "K" => Self::Kelvin,
            "ms" => linear("ms", Dimension::Duration, 0.001),
            "s" => linear("s", Dimension::Duration, 1.0),
            "min" => linear("min", Dimension::Duration, 60.0),
            "h" => linear("h", Dimension::Duration, 3_600.0),
            "d" => linear("d", Dimension::Duration, 86_400.0),
            "mm" => linear("mm", Dimension::Length, 0.001),
            "cm" => linear("cm", Dimension::Length, 0.01),
            "m" => linear("m", Dimension::Length, 1.0),
            "km" => linear("km", Dimension::Length, 1_000.0),
            "in" => linear("in", Dimension::Length, 0.0254),
            "ft" => linear("ft", Dimension::Length, 0.3048),
            "mi" => linear("mi", Dimension::Length, 1_609.344),
            _ => return None,
        })
    }

    fn dimension(self) -> Dimension {
        match self {
            Self::Linear { dimension, .. } => dimension,
            Self::Celsius | Self::Fahrenheit | Self::Kelvin => Dimension::Temperature,
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Self::Linear { symbol, .. } => symbol,
            Self::Celsius => "C",
            Self::Fahrenheit => "F",
            Self::Kelvin => "K",
        }
    }
}

fn convert_units(value: f64, from: Unit, to: Unit) -> Result<f64, CalcError> {
    let canonical = match from {
        Unit::Linear { factor, .. } => value * factor,
        Unit::Celsius => value + 273.15,
        Unit::Fahrenheit => (value - 32.0) * 5.0 / 9.0 + 273.15,
        Unit::Kelvin => value,
    };
    if from.dimension() == Dimension::Temperature && canonical < 0.0 {
        return Err(CalcError::Invalid(
            "temperature is below absolute zero".into(),
        ));
    }
    let result = match to {
        Unit::Linear { factor, .. } => canonical / factor,
        Unit::Celsius => canonical - 273.15,
        Unit::Fahrenheit => (canonical - 273.15) * 9.0 / 5.0 + 32.0,
        Unit::Kelvin => canonical,
    };
    finite(result)
}

fn calculate_base(input: &str) -> Result<Calculation, CalcError> {
    let fields = input.split_ascii_whitespace().collect::<Vec<_>>();
    if fields.len() != 2 {
        return Err(CalcError::Invalid(
            "expected: base <integer> <2|8|10|16>".into(),
        ));
    }
    let value = parse_integer(fields[0])?;
    let radix = fields[1]
        .parse::<u32>()
        .ok()
        .filter(|radix| matches!(radix, 2 | 8 | 10 | 16))
        .ok_or_else(|| CalcError::Invalid("base must be 2, 8, 10, or 16".into()))?;
    let result = format_integer_radix(value, radix);
    Ok(Calculation {
        result,
        integer: Some(value),
    })
}

fn parse_integer(raw: &str) -> Result<i64, CalcError> {
    let (negative, unsigned) = raw
        .strip_prefix('-')
        .map(|rest| (true, rest))
        .or_else(|| raw.strip_prefix('+').map(|rest| (false, rest)))
        .unwrap_or((false, raw));
    let (radix, digits) = if let Some(rest) = unsigned.strip_prefix("0x") {
        (16, rest)
    } else if let Some(rest) = unsigned.strip_prefix("0o") {
        (8, rest)
    } else if let Some(rest) = unsigned.strip_prefix("0b") {
        (2, rest)
    } else {
        (10, unsigned)
    };
    validate_underscores(digits)?;
    let digits = digits.replace('_', "");
    if digits.is_empty() {
        return Err(CalcError::Invalid("missing integer digits".into()));
    }
    let magnitude = i128::from_str_radix(&digits, radix)
        .map_err(|_| CalcError::Invalid("integer is invalid or out of range".into()))?;
    let signed = if negative { -magnitude } else { magnitude };
    i64::try_from(signed)
        .map_err(|_| CalcError::Invalid("integer is invalid or out of range".into()))
}

fn format_integer_radix(value: i64, radix: u32) -> String {
    let negative = value < 0;
    let magnitude = value.unsigned_abs();
    let digits = match radix {
        2 => format!("{magnitude:b}"),
        8 => format!("{magnitude:o}"),
        10 => magnitude.to_string(),
        16 => format!("{magnitude:X}"),
        _ => unreachable!(),
    };
    let prefix = match radix {
        2 => "0b",
        8 => "0o",
        10 => "",
        16 => "0x",
        _ => unreachable!(),
    };
    format!("{}{prefix}{digits}", if negative { "-" } else { "" })
}

fn calculate_unix(input: &str) -> Result<Calculation, CalcError> {
    let seconds = parse_integer(input.trim())?;
    let date = DateTime::<Utc>::from_timestamp(seconds, 0)
        .ok_or_else(|| CalcError::Invalid("Unix timestamp is out of range".into()))?;
    Ok(Calculation {
        result: date.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        integer: None,
    })
}

fn calculate_date(input: &str) -> Result<Calculation, CalcError> {
    let fields = input.split_ascii_whitespace().collect::<Vec<_>>();
    let (date_raw, sign, days_raw) = match fields.as_slice() {
        [date, signed] if signed.starts_with(['+', '-']) => (&**date, &signed[..1], &signed[1..]),
        [date, sign @ ("+" | "-"), days] => (&**date, *sign, &**days),
        _ => {
            return Err(CalcError::Invalid(
                "expected: date <YYYY-MM-DD> +/- <N>d".into(),
            ))
        }
    };
    let date = NaiveDate::parse_from_str(date_raw, "%Y-%m-%d")
        .map_err(|_| CalcError::Invalid("invalid ISO date".into()))?;
    let digits = days_raw
        .strip_suffix('d')
        .ok_or_else(|| CalcError::Invalid("date offset must end in d".into()))?;
    let days = digits
        .parse::<u64>()
        .map_err(|_| CalcError::Invalid("invalid date offset".into()))?;
    let result = if sign == "+" {
        date.checked_add_days(Days::new(days))
    } else {
        date.checked_sub_days(Days::new(days))
    }
    .ok_or_else(|| CalcError::Invalid("date result is out of range".into()))?;
    Ok(Calculation {
        result: result.format("%Y-%m-%d").to_string(),
        integer: None,
    })
}

fn parse_decimal(raw: &str) -> Result<f64, CalcError> {
    validate_underscores(raw)?;
    let normalized = raw.replace('_', "");
    let value = normalized
        .parse::<f64>()
        .map_err(|_| CalcError::Invalid("invalid decimal number".into()))?;
    finite(value)
}

fn validate_underscores(raw: &str) -> Result<(), CalcError> {
    for (index, byte) in raw.as_bytes().iter().enumerate() {
        if *byte == b'_'
            && (index == 0
                || index + 1 == raw.len()
                || !raw.as_bytes()[index - 1].is_ascii_digit()
                || !raw.as_bytes()[index + 1].is_ascii_digit())
        {
            return Err(CalcError::Invalid(
                "underscores must appear between digits".into(),
            ));
        }
    }
    Ok(())
}

fn finite(value: f64) -> Result<f64, CalcError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(CalcError::Invalid("result is not finite".into()))
    }
}

fn exact_i64(value: f64) -> Option<i64> {
    // `i64::MAX as f64` rounds up to 2^63, so the upper bound must stay
    // exclusive or Rust's float-to-int saturation would misclassify 2^63.
    const I64_EXCLUSIVE_MAX: f64 = 9_223_372_036_854_775_808.0;
    (value.fract() == 0.0 && value >= i64::MIN as f64 && value < I64_EXCLUSIVE_MAX)
        .then_some(value as i64)
}

fn format_number(value: f64) -> Result<String, CalcError> {
    let value = finite(value)?;
    let value = if value == 0.0 { 0.0 } else { value };
    let mut output = if value != 0.0 && (value.abs() >= 1e15 || value.abs() < 1e-9) {
        format!("{value:.12e}")
    } else {
        format!("{value:.12}")
    };
    if let Some((mantissa, exponent)) = output.split_once('e') {
        let mantissa = mantissa
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string();
        output = format!("{mantissa}e{exponent}");
    } else {
        output = output.trim_end_matches('0').trim_end_matches('.').into();
    }
    if output.is_empty() || output == "-" {
        output = "0".into();
    }
    Ok(output)
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,
    LParen,
    RParen,
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    nesting: usize,
}

impl Parser {
    fn new(input: &str) -> Result<Self, CalcError> {
        if input.is_empty() {
            return Err(CalcError::Invalid("expression is empty".into()));
        }
        let tokens = tokenize(input)?;
        Ok(Self {
            tokens,
            cursor: 0,
            nesting: 0,
        })
    }

    fn parse(mut self) -> Result<f64, CalcError> {
        let value = self.expression()?;
        if self.cursor != self.tokens.len() {
            return Err(CalcError::Invalid("unexpected token".into()));
        }
        finite(value)
    }

    fn expression(&mut self) -> Result<f64, CalcError> {
        let mut left = self.term()?;
        loop {
            if self.take(&Token::Plus) {
                left = finite(left + self.term()?)?;
            } else if self.take(&Token::Minus) {
                left = finite(left - self.term()?)?;
            } else {
                return Ok(left);
            }
        }
    }

    fn term(&mut self) -> Result<f64, CalcError> {
        let mut left = self.unary()?;
        loop {
            if self.take(&Token::Star) {
                left = finite(left * self.unary()?)?;
            } else if self.take(&Token::Slash) {
                let right = self.unary()?;
                if right == 0.0 {
                    return Err(CalcError::Invalid("division by zero".into()));
                }
                left = finite(left / right)?;
            } else {
                return Ok(left);
            }
        }
    }

    fn unary(&mut self) -> Result<f64, CalcError> {
        if self.take(&Token::Plus) {
            self.unary()
        } else if self.take(&Token::Minus) {
            finite(-self.unary()?)
        } else {
            self.power()
        }
    }

    fn power(&mut self) -> Result<f64, CalcError> {
        let base = self.postfix()?;
        if !self.take(&Token::Caret) {
            return Ok(base);
        }
        let exponent = self.unary()?;
        if exponent.abs() > MAX_POWER_MAGNITUDE {
            return Err(CalcError::Limit(format!(
                "exponent magnitude exceeds {MAX_POWER_MAGNITUDE}"
            )));
        }
        finite(base.powf(exponent))
    }

    fn postfix(&mut self) -> Result<f64, CalcError> {
        let mut value = self.primary()?;
        while self.take(&Token::Percent) {
            value = finite(value / 100.0)?;
        }
        Ok(value)
    }

    fn primary(&mut self) -> Result<f64, CalcError> {
        match self.tokens.get(self.cursor).cloned() {
            Some(Token::Number(value)) => {
                self.cursor += 1;
                Ok(value)
            }
            Some(Token::LParen) => {
                if self.nesting >= MAX_NESTING {
                    return Err(CalcError::Limit(format!(
                        "nesting exceeds {MAX_NESTING} levels"
                    )));
                }
                self.cursor += 1;
                self.nesting += 1;
                let value = self.expression()?;
                self.nesting -= 1;
                if !self.take(&Token::RParen) {
                    return Err(CalcError::Invalid("missing closing parenthesis".into()));
                }
                Ok(value)
            }
            _ => Err(CalcError::Invalid(
                "expected a number or parenthesis".into(),
            )),
        }
    }

    fn take(&mut self, expected: &Token) -> bool {
        if self.tokens.get(self.cursor) == Some(expected) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>, CalcError> {
    let bytes = input.as_bytes();
    let mut cursor = 0;
    let mut tokens = Vec::new();
    while cursor < bytes.len() {
        if bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        let token = match bytes[cursor] {
            b'+' => {
                cursor += 1;
                Token::Plus
            }
            b'-' => {
                cursor += 1;
                Token::Minus
            }
            b'*' => {
                cursor += 1;
                Token::Star
            }
            b'/' => {
                cursor += 1;
                Token::Slash
            }
            b'^' => {
                cursor += 1;
                Token::Caret
            }
            b'%' => {
                cursor += 1;
                Token::Percent
            }
            b'(' => {
                cursor += 1;
                Token::LParen
            }
            b')' => {
                cursor += 1;
                Token::RParen
            }
            byte if byte.is_ascii_digit() || byte == b'.' => {
                let start = cursor;
                cursor += 1;
                while cursor < bytes.len()
                    && (bytes[cursor].is_ascii_digit()
                        || matches!(bytes[cursor], b'.' | b'_' | b'e' | b'E')
                        || (matches!(bytes[cursor], b'+' | b'-')
                            && matches!(bytes[cursor - 1], b'e' | b'E')))
                {
                    cursor += 1;
                }
                Token::Number(parse_decimal(&input[start..cursor])?)
            }
            _ => {
                return Err(CalcError::Invalid(
                    "expression contains an invalid token".into(),
                ))
            }
        };
        tokens.push(token);
        if tokens.len() > MAX_TOKENS {
            return Err(CalcError::Limit(format!(
                "expression exceeds {MAX_TOKENS} tokens"
            )));
        }
    }
    if tokens.is_empty() {
        return Err(CalcError::Invalid("expression is empty".into()));
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use luma_application::{FakePasteboard, PasteboardError};
    use luma_domain::{ActionDescriptor, ResultId};
    use std::sync::Mutex;

    fn result(input: &str) -> String {
        calculate(input).unwrap().result
    }

    #[test]
    fn arithmetic_precedence_associativity_unary_and_percent() {
        assert_eq!(result("2 + 3 * 4"), "14");
        assert_eq!(result("2 ^ 3 ^ 2"), "512");
        assert_eq!(result("-2 ^ 2"), "-4");
        assert_eq!(result("2 ^ -2"), "0.25");
        assert_eq!(result("(2 + 3) * 4"), "20");
        assert_eq!(result("50%"), "0.5");
    }

    #[test]
    fn arithmetic_rejects_limits_and_invalid_results() {
        assert!(calculate("1 / 0").is_err());
        assert!(calculate("2 ^ 1025").is_err());
        assert!(calculate(&"(".repeat(MAX_NESTING + 1)).is_err());
        assert!(calculate(&"1+".repeat(MAX_TOKENS)).is_err());
        assert!(calculate(&"1".repeat(MAX_INPUT_CHARS + 1)).is_err());
        assert!(calculate("1e309").is_err());
        assert!(calculate("1__0 + 2").is_err());
    }

    #[test]
    fn integer_actions_respect_exact_i64_boundaries() {
        assert_eq!(calculate("2 ^ 63").unwrap().integer, None);
        assert_eq!(calculate("-2 ^ 63").unwrap().integer, Some(i64::MIN));
    }

    #[test]
    fn converts_every_supported_linear_unit_family() {
        let cases = [
            ("1 KB in B", "1000 B"),
            ("1 KiB in B", "1024 B"),
            ("1 TB in GB", "1000 GB"),
            ("1 TiB in GiB", "1024 GiB"),
            ("1000 ms in s", "1 s"),
            ("1 d in h", "24 h"),
            ("10 mm in cm", "1 cm"),
            ("1 km in m", "1000 m"),
            ("1 ft in in", "12 in"),
            ("1 mi in km", "1.609344 km"),
        ];
        for (input, expected) in cases {
            assert_eq!(result(input), expected, "{input}");
        }
    }

    #[test]
    fn converts_temperature_offsets_and_rejects_dimensions() {
        assert_eq!(result("0 C in F"), "32 F");
        assert_eq!(result("32 F in C"), "0 C");
        assert_eq!(result("273.15 K in C"), "0 C");
        assert!(calculate("-1 K in C").is_err());
        assert!(calculate("1 m in s").is_err());
    }

    #[test]
    fn base_and_date_helpers_are_deterministic() {
        assert_eq!(result("base 0xff 10"), "255");
        assert_eq!(result("base -10 2"), "-0b1010");
        assert_eq!(result("unix -1"), "1969-12-31T23:59:59Z");
        assert_eq!(result("date 2024-02-28 + 1d"), "2024-02-29");
        assert_eq!(result("date 2024-02-29 +1d"), "2024-03-01");
        assert_eq!(result("date 2024-03-01 - 1d"), "2024-02-29");
        assert!(calculate("date 2023-02-29 + 1d").is_err());
    }

    #[test]
    fn strict_global_detector_rejects_non_expressions() {
        let rejected = [
            "calculator",
            "hello world",
            "/Users/me/project",
            "project-2",
            "v1.2.3",
            "1.2.3",
            "2026-07-27",
            "openssl@3",
            "README.md",
            "42",
        ];
        for input in rejected {
            assert!(!is_strict_global_expression(input), "{input}");
        }
        for input in ["1 + 2", "128 MiB in GiB", "(3 * 4)", "50%"] {
            assert!(is_strict_global_expression(input), "{input}");
        }
    }

    fn search_item(input: &str) -> SearchItem {
        calculation_item(input, calculate(input).unwrap()).into_domain()
    }

    #[tokio::test]
    async fn copy_result_honors_pre_cancel() {
        let pasteboard = Arc::new(FakePasteboard::new());
        let module = CalculatorModule::with_deps(pasteboard.clone());
        let cancel = CancellationToken::new();
        cancel.cancel();
        let outcome = module
            .perform(
                ActionRequest {
                    result: search_item("1 + 2"),
                    action: safe_action("copy_result", "Copy result"),
                    confirmation: false,
                },
                cancel,
            )
            .await;
        assert_eq!(outcome, ActionOutcome::Cancelled);
        assert_eq!(pasteboard.last_text(), None);
    }

    struct FailingPasteboard {
        writes: Mutex<usize>,
    }

    #[async_trait]
    impl PasteboardPort for FailingPasteboard {
        async fn read_text(&self) -> Result<Option<String>, PasteboardError> {
            Ok(None)
        }

        async fn write_text(&self, _text: &str) -> Result<(), PasteboardError> {
            *self.writes.lock().unwrap() += 1;
            Err(PasteboardError::Unavailable("fixture".into()))
        }
    }

    #[tokio::test]
    async fn pasteboard_failure_is_not_reported_as_success() {
        let module = CalculatorModule::with_deps(Arc::new(FailingPasteboard {
            writes: Mutex::new(0),
        }));
        let outcome = module
            .perform(
                ActionRequest {
                    result: search_item("1 + 2"),
                    action: ActionDescriptor {
                        id: ActionId::new("copy_result"),
                        label: "Copy".into(),
                        risk: ActionRisk::Safe,
                        confirmation: false,
                    },
                    confirmation: false,
                },
                CancellationToken::new(),
            )
            .await;
        assert!(matches!(outcome, ActionOutcome::Failed { .. }));
    }

    #[test]
    fn stable_result_ids_do_not_depend_on_output_format() {
        let one = calculation_item("1 + 2", calculate("1 + 2").unwrap());
        let mut changed = calculate("1 + 2").unwrap();
        changed.result = "3.0".into();
        let two = calculation_item("1 + 2", changed);
        assert_eq!(one.id, two.id);
    }

    #[test]
    fn helper_for_test_constructs_domain_identity() {
        let item = search_item("2 + 2");
        assert_eq!(
            item.id,
            ResultId::new(format!("calc:{:016x}", stable_hash("2 + 2")))
        );
    }
}
