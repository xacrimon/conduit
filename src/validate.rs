use std::collections::BTreeMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub code: &'static str,
    pub message: Option<String>,
    pub params: Vec<(&'static str, String)>,
}

impl ValidationError {
    pub fn new(code: &'static str) -> Self {
        Self {
            code,
            message: None,
            params: Vec::new(),
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn add_param(mut self, name: &'static str, value: impl fmt::Display) -> Self {
        self.params.push((name, value.to_string()));
        self
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref msg) = self.message {
            write!(f, "{}", msg)
        } else {
            write!(f, "Validation error: {}", self.code)
        }
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug, Default)]
pub struct ValidationErrors {
    errors: BTreeMap<&'static str, Vec<ValidationError>>,
}

impl ValidationErrors {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, field: &'static str, error: ValidationError) {
        self.errors.entry(field).or_default().push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn field_errors(&self) -> &BTreeMap<&'static str, Vec<ValidationError>> {
        &self.errors
    }

    pub fn into_result(self) -> Result<(), Self> {
        if self.is_empty() { Ok(()) } else { Err(self) }
    }

    pub fn merge_self(&mut self, field: &'static str, other: Result<(), ValidationErrors>) {
        if let Err(other) = other {
            for (_, errs) in other.errors {
                for e in errs {
                    self.add(field, e);
                }
            }
        }
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (field, errors) in &self.errors {
            for error in errors {
                write!(f, "{}: {}\n", field, error)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

// ---------------------------------------------------------------------------
// Core trait
// ---------------------------------------------------------------------------

pub trait Validate {
    fn validate(&self) -> Result<(), ValidationErrors>;
}

// ---------------------------------------------------------------------------
// ValidateLength
// ---------------------------------------------------------------------------

pub trait ValidateLength {
    fn length(&self) -> usize;

    fn validate_length(
        &self,
        min: Option<usize>,
        max: Option<usize>,
        equal: Option<usize>,
    ) -> bool {
        let len = self.length();
        if let Some(eq) = equal {
            return len == eq;
        }
        if let Some(min) = min {
            if len < min {
                return false;
            }
        }
        if let Some(max) = max {
            if len > max {
                return false;
            }
        }
        true
    }
}

impl ValidateLength for str {
    fn length(&self) -> usize {
        self.chars().count()
    }
}

impl ValidateLength for String {
    fn length(&self) -> usize {
        self.chars().count()
    }
}

impl<T> ValidateLength for Vec<T> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T> ValidateLength for &T
where
    T: ValidateLength + ?Sized,
{
    fn length(&self) -> usize {
        (**self).length()
    }
}

impl<T: ValidateLength> ValidateLength for Option<T> {
    fn length(&self) -> usize {
        match self {
            Some(v) => v.length(),
            None => 0,
        }
    }

    fn validate_length(
        &self,
        min: Option<usize>,
        max: Option<usize>,
        equal: Option<usize>,
    ) -> bool {
        match self {
            Some(v) => v.validate_length(min, max, equal),
            None => true,
        }
    }
}

// ---------------------------------------------------------------------------
// ValidateRange
// ---------------------------------------------------------------------------

pub trait ValidateRange {
    fn validate_range(
        &self,
        min: Option<f64>,
        max: Option<f64>,
        exclusive_min: Option<f64>,
        exclusive_max: Option<f64>,
    ) -> bool;
}

macro_rules! impl_validate_range {
    ($($ty:ty),*) => {
        $(
            impl ValidateRange for $ty {
                fn validate_range(
                    &self,
                    min: Option<f64>,
                    max: Option<f64>,
                    exclusive_min: Option<f64>,
                    exclusive_max: Option<f64>,
                ) -> bool {
                    let val = *self as f64;
                    if let Some(min) = min {
                        if val < min { return false; }
                    }
                    if let Some(max) = max {
                        if val > max { return false; }
                    }
                    if let Some(emin) = exclusive_min {
                        if val <= emin { return false; }
                    }
                    if let Some(emax) = exclusive_max {
                        if val >= emax { return false; }
                    }
                    true
                }
            }
        )*
    };
}

impl_validate_range!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);

impl<T: ValidateRange> ValidateRange for Option<T> {
    fn validate_range(
        &self,
        min: Option<f64>,
        max: Option<f64>,
        exclusive_min: Option<f64>,
        exclusive_max: Option<f64>,
    ) -> bool {
        match self {
            Some(v) => v.validate_range(min, max, exclusive_min, exclusive_max),
            None => true,
        }
    }
}

// ---------------------------------------------------------------------------
// ValidateEmail (simplified HTML5 spec)
// ---------------------------------------------------------------------------

pub trait ValidateEmail {
    fn validate_email(&self) -> bool;
}

impl ValidateEmail for str {
    fn validate_email(&self) -> bool {
        validate_email_str(self)
    }
}

impl ValidateEmail for String {
    fn validate_email(&self) -> bool {
        validate_email_str(self)
    }
}

impl<T: ValidateEmail + ?Sized> ValidateEmail for &T {
    fn validate_email(&self) -> bool {
        (**self).validate_email()
    }
}

impl<T: ValidateEmail> ValidateEmail for Option<T> {
    fn validate_email(&self) -> bool {
        match self {
            Some(v) => v.validate_email(),
            None => true,
        }
    }
}

fn validate_email_str(val: &str) -> bool {
    let Some((user, domain)) = val.rsplit_once('@') else {
        return false;
    };
    if user.is_empty() || user.len() > 64 {
        return false;
    }
    if domain.is_empty() || domain.len() > 255 {
        return false;
    }

    let user_re = crate::utils::re!(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+$");
    if !user_re.is_match(user) {
        return false;
    }

    let domain_re = crate::utils::re!(
        r"^(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)*[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?$"
    );
    if !domain_re.is_match(domain) {
        let ip_re = crate::utils::re!(r"^\[(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\]$");
        if !ip_re.is_match(domain) {
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------
// ValidateUrl
// ---------------------------------------------------------------------------

pub trait ValidateUrl {
    fn validate_url(&self) -> bool;
}

impl ValidateUrl for str {
    fn validate_url(&self) -> bool {
        url::Url::parse(self).is_ok()
    }
}

impl ValidateUrl for String {
    fn validate_url(&self) -> bool {
        self.as_str().validate_url()
    }
}

impl<T: ValidateUrl + ?Sized> ValidateUrl for &T {
    fn validate_url(&self) -> bool {
        (**self).validate_url()
    }
}

impl<T: ValidateUrl> ValidateUrl for Option<T> {
    fn validate_url(&self) -> bool {
        match self {
            Some(v) => v.validate_url(),
            None => true,
        }
    }
}

// ---------------------------------------------------------------------------
// ValidateContains / ValidateDoesNotContain
// ---------------------------------------------------------------------------

pub trait ValidateContains {
    fn validate_contains(&self, needle: &str) -> bool;
}

pub trait ValidateDoesNotContain {
    fn validate_does_not_contain(&self, needle: &str) -> bool;
}

impl ValidateContains for str {
    fn validate_contains(&self, needle: &str) -> bool {
        self.contains(needle)
    }
}

impl ValidateContains for String {
    fn validate_contains(&self, needle: &str) -> bool {
        self.as_str().contains(needle)
    }
}

impl<T: ValidateContains + ?Sized> ValidateContains for &T {
    fn validate_contains(&self, needle: &str) -> bool {
        (**self).validate_contains(needle)
    }
}

impl<T: ValidateContains> ValidateContains for Option<T> {
    fn validate_contains(&self, needle: &str) -> bool {
        match self {
            Some(v) => v.validate_contains(needle),
            None => true,
        }
    }
}

impl ValidateDoesNotContain for str {
    fn validate_does_not_contain(&self, needle: &str) -> bool {
        !self.contains(needle)
    }
}

impl ValidateDoesNotContain for String {
    fn validate_does_not_contain(&self, needle: &str) -> bool {
        !self.as_str().contains(needle)
    }
}

impl<T: ValidateDoesNotContain + ?Sized> ValidateDoesNotContain for &T {
    fn validate_does_not_contain(&self, needle: &str) -> bool {
        (**self).validate_does_not_contain(needle)
    }
}

impl<T: ValidateDoesNotContain> ValidateDoesNotContain for Option<T> {
    fn validate_does_not_contain(&self, needle: &str) -> bool {
        match self {
            Some(v) => v.validate_does_not_contain(needle),
            None => true,
        }
    }
}

// ---------------------------------------------------------------------------
// ValidateRegex
// ---------------------------------------------------------------------------

pub trait ValidateRegex {
    fn validate_regex(&self, regex: &regex_lite::Regex) -> bool;
}

impl ValidateRegex for str {
    fn validate_regex(&self, regex: &regex_lite::Regex) -> bool {
        regex.is_match(self)
    }
}

impl ValidateRegex for String {
    fn validate_regex(&self, regex: &regex_lite::Regex) -> bool {
        regex.is_match(self)
    }
}

impl<T: ValidateRegex + ?Sized> ValidateRegex for &T {
    fn validate_regex(&self, regex: &regex_lite::Regex) -> bool {
        (**self).validate_regex(regex)
    }
}

impl<T: ValidateRegex> ValidateRegex for Option<T> {
    fn validate_regex(&self, regex: &regex_lite::Regex) -> bool {
        match self {
            Some(v) => v.validate_regex(regex),
            None => true,
        }
    }
}

// ---------------------------------------------------------------------------
// ValidateRequired
// ---------------------------------------------------------------------------

pub trait ValidateRequired {
    fn validate_required(&self) -> bool;
}

impl<T> ValidateRequired for Option<T> {
    fn validate_required(&self) -> bool {
        self.is_some()
    }
}

// ---------------------------------------------------------------------------
// ValidateNonControlCharacter
// ---------------------------------------------------------------------------

pub trait ValidateNonControlCharacter {
    fn validate_non_control_character(&self) -> bool;
}

impl ValidateNonControlCharacter for str {
    fn validate_non_control_character(&self) -> bool {
        self.chars().all(|c| !c.is_control())
    }
}

impl ValidateNonControlCharacter for String {
    fn validate_non_control_character(&self) -> bool {
        self.as_str().validate_non_control_character()
    }
}

impl<T: ValidateNonControlCharacter + ?Sized> ValidateNonControlCharacter for &T {
    fn validate_non_control_character(&self) -> bool {
        (**self).validate_non_control_character()
    }
}

impl<T: ValidateNonControlCharacter> ValidateNonControlCharacter for Option<T> {
    fn validate_non_control_character(&self) -> bool {
        match self {
            Some(v) => v.validate_non_control_character(),
            None => true,
        }
    }
}

// ---------------------------------------------------------------------------
// ValidateIp
// ---------------------------------------------------------------------------

pub trait ValidateIp {
    fn validate_ip(&self) -> bool;
}

impl ValidateIp for str {
    fn validate_ip(&self) -> bool {
        self.parse::<std::net::IpAddr>().is_ok()
    }
}

impl ValidateIp for String {
    fn validate_ip(&self) -> bool {
        self.as_str().validate_ip()
    }
}

impl<T: ValidateIp + ?Sized> ValidateIp for &T {
    fn validate_ip(&self) -> bool {
        (**self).validate_ip()
    }
}

impl<T: ValidateIp> ValidateIp for Option<T> {
    fn validate_ip(&self) -> bool {
        match self {
            Some(v) => v.validate_ip(),
            None => true,
        }
    }
}

// ---------------------------------------------------------------------------
// must_match helper (used by generated code)
// ---------------------------------------------------------------------------

pub fn validate_must_match<T: PartialEq>(a: &T, b: &T) -> bool {
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;
    use conduit_derive::Validate;

    #[derive(Validate)]
    struct TestForm {
        #[validate(length(min = 1, max = 100))]
        #[validate(non_control_character)]
        username: String,

        #[validate(email)]
        email: String,

        #[validate(length(min = 8))]
        password: String,

        #[validate(must_match(other = "password"))]
        password_confirm: String,

        #[validate(range(min = 0, max = 150))]
        age: u8,

        #[validate(url)]
        website: Option<String>,

        #[validate(contains(pattern = "@"))]
        contact: String,

        #[validate(required)]
        bio: Option<String>,
    }

    #[test]
    fn valid_form() {
        let form = TestForm {
            username: "alice".into(),
            email: "alice@example.com".into(),
            password: "hunter42!".into(),
            password_confirm: "hunter42!".into(),
            age: 25,
            website: Some("https://example.com".into()),
            contact: "hello@world".into(),
            bio: Some("Hi there".into()),
        };
        assert!(form.validate().is_ok());
    }

    #[test]
    fn invalid_email() {
        let form = TestForm {
            username: "alice".into(),
            email: "not-an-email".into(),
            password: "hunter42!".into(),
            password_confirm: "hunter42!".into(),
            age: 25,
            website: None,
            contact: "hello@world".into(),
            bio: Some("Hi".into()),
        };
        let err = form.validate().unwrap_err();
        assert!(err.field_errors().contains_key("email"));
    }

    #[test]
    fn password_mismatch() {
        let form = TestForm {
            username: "alice".into(),
            email: "alice@example.com".into(),
            password: "hunter42!".into(),
            password_confirm: "wrong".into(),
            age: 25,
            website: None,
            contact: "hello@world".into(),
            bio: Some("Hi".into()),
        };
        let err = form.validate().unwrap_err();
        assert!(err.field_errors().contains_key("password_confirm"));
    }

    #[test]
    fn length_violations() {
        let form = TestForm {
            username: "".into(),
            email: "alice@example.com".into(),
            password: "short".into(),
            password_confirm: "short".into(),
            age: 25,
            website: None,
            contact: "hello@world".into(),
            bio: Some("Hi".into()),
        };
        let err = form.validate().unwrap_err();
        assert!(err.field_errors().contains_key("username"));
        assert!(err.field_errors().contains_key("password"));
    }

    #[test]
    fn range_out_of_bounds() {
        let form = TestForm {
            username: "alice".into(),
            email: "alice@example.com".into(),
            password: "hunter42!".into(),
            password_confirm: "hunter42!".into(),
            age: 200,
            website: None,
            contact: "hello@world".into(),
            bio: Some("Hi".into()),
        };
        let err = form.validate().unwrap_err();
        assert!(err.field_errors().contains_key("age"));
    }

    #[test]
    fn required_none() {
        let form = TestForm {
            username: "alice".into(),
            email: "alice@example.com".into(),
            password: "hunter42!".into(),
            password_confirm: "hunter42!".into(),
            age: 25,
            website: None,
            contact: "hello@world".into(),
            bio: None,
        };
        let err = form.validate().unwrap_err();
        assert!(err.field_errors().contains_key("bio"));
    }
}
