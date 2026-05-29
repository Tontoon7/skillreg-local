use reqwest::StatusCode;
use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
struct ParsedApiError {
    code: Option<String>,
    message: String,
}

pub fn format_api_error(action: &str, status: StatusCode, body: &str) -> String {
    let parsed = parse_api_error(body);
    let message = parsed
        .as_ref()
        .map(|error| error.message.as_str())
        .unwrap_or_else(|| body.trim());

    if status == StatusCode::PAYMENT_REQUIRED {
        if parsed
            .as_ref()
            .and_then(|error| error.code.as_deref())
            .is_some_and(|code| code == "PLAN_UPGRADE_REQUIRED")
        {
            return format!("{}: Upgrade required. {}", action, message);
        }

        return format!("{}: Plan limit reached. {}", action, message);
    }

    if message.is_empty() {
        return format!("{} {}", action, status.as_u16());
    }

    format!("{} {}: {}", action, status.as_u16(), message)
}

fn parse_api_error(body: &str) -> Option<ParsedApiError> {
    let value: Value = serde_json::from_str(body).ok()?;
    let error = value.get("error")?;

    if let Some(message) = error.as_str() {
        return Some(ParsedApiError {
            code: None,
            message: message.to_string(),
        });
    }

    let message = error.get("message")?.as_str()?.to_string();
    let code = error
        .get("code")
        .and_then(|code| code.as_str())
        .map(ToString::to_string);

    Some(ParsedApiError { code, message })
}

#[cfg(test)]
mod tests {
    use super::format_api_error;
    use reqwest::StatusCode;

    #[test]
    fn formats_structured_plan_upgrade_errors() {
        let body = r#"{"error":{"code":"PLAN_UPGRADE_REQUIRED","message":"Approval workflows require the Enterprise plan."}}"#;

        assert_eq!(
            format_api_error("Publish failed", StatusCode::PAYMENT_REQUIRED, body),
            "Publish failed: Upgrade required. Approval workflows require the Enterprise plan."
        );
    }

    #[test]
    fn formats_legacy_payment_errors_as_plan_limits() {
        let body = r#"{"error":"Skill limit reached (10 on free plan)"}"#;

        assert_eq!(
            format_api_error("Publish failed", StatusCode::PAYMENT_REQUIRED, body),
            "Publish failed: Plan limit reached. Skill limit reached (10 on free plan)"
        );
    }

    #[test]
    fn formats_plain_json_errors_without_raw_payload() {
        let body = r#"{"error":"Skill not found"}"#;

        assert_eq!(
            format_api_error("Skill lookup failed", StatusCode::NOT_FOUND, body),
            "Skill lookup failed 404: Skill not found"
        );
    }

    #[test]
    fn keeps_raw_non_json_errors_as_fallback() {
        assert_eq!(
            format_api_error("API error", StatusCode::BAD_GATEWAY, "upstream unavailable"),
            "API error 502: upstream unavailable"
        );
    }
}
