use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Expr, Fields, Lit, MetaNameValue, Token, parse_macro_input};

#[proc_macro_derive(Validate, attributes(validate))]
pub fn derive_validation(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_validate(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn impl_validate(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "Validate can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "Validate can only be derived for structs",
            ));
        }
    };

    let mut field_validations = Vec::new();

    for field in fields {
        let field_ident = field.ident.as_ref().unwrap();
        let field_name = field_ident.to_string();

        for attr in &field.attrs {
            if !attr.path().is_ident("validate") {
                continue;
            }

            let validators = parse_validate_attr(attr)?;
            for validator in validators {
                let tokens =
                    generate_validation(&validator, field_ident, &field_name, field.span())?;
                field_validations.push(tokens);
            }
        }
    }

    Ok(quote! {
        impl #impl_generics crate::validate::Validate for #name #ty_generics #where_clause {
            fn validate(&self) -> ::std::result::Result<(), crate::validate::ValidationErrors> {
                let mut errors = crate::validate::ValidationErrors::new();
                #(#field_validations)*
                errors.into_result()
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Attribute parsing
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Validator {
    Email {
        message: Option<String>,
    },
    Url {
        message: Option<String>,
    },
    Ip {
        message: Option<String>,
    },
    NonControlCharacter {
        message: Option<String>,
    },
    Required {
        message: Option<String>,
    },
    Length {
        min: Option<Expr>,
        max: Option<Expr>,
        equal: Option<Expr>,
        message: Option<String>,
    },
    Range {
        min: Option<Expr>,
        max: Option<Expr>,
        exclusive_min: Option<Expr>,
        exclusive_max: Option<Expr>,
        message: Option<String>,
    },
    Contains {
        pattern: String,
        message: Option<String>,
    },
    DoesNotContain {
        pattern: String,
        message: Option<String>,
    },
    MustMatch {
        other: String,
        message: Option<String>,
    },
    Regex {
        path: Expr,
        message: Option<String>,
    },
    Nested,
    Custom {
        function: syn::Path,
        message: Option<String>,
    },
}

fn parse_validate_attr(attr: &syn::Attribute) -> syn::Result<Vec<Validator>> {
    let mut validators = Vec::new();

    attr.parse_nested_meta(|meta| {
        let ident = meta
            .path
            .get_ident()
            .ok_or_else(|| syn::Error::new_spanned(&meta.path, "expected validator name"))?;
        let name = ident.to_string();

        match name.as_str() {
            "email" => {
                let message = parse_optional_message(&meta)?;
                validators.push(Validator::Email { message });
            }
            "url" => {
                let message = parse_optional_message(&meta)?;
                validators.push(Validator::Url { message });
            }
            "ip" => {
                let message = parse_optional_message(&meta)?;
                validators.push(Validator::Ip { message });
            }
            "non_control_character" => {
                let message = parse_optional_message(&meta)?;
                validators.push(Validator::NonControlCharacter { message });
            }
            "required" => {
                let message = parse_optional_message(&meta)?;
                validators.push(Validator::Required { message });
            }
            "nested" => {
                validators.push(Validator::Nested);
            }
            "length" => {
                let mut min = None;
                let mut max = None;
                let mut equal = None;
                let mut message = None;

                parse_kv_args(&meta, |key, value| {
                    match key.as_str() {
                        "min" => min = Some(value),
                        "max" => max = Some(value),
                        "equal" => equal = Some(value),
                        "message" => {
                            message = Some(expr_to_string(&value)?);
                        }
                        _ => {
                            return Err(syn::Error::new_spanned(
                                &value,
                                format!("unknown length parameter: {}", key),
                            ));
                        }
                    }
                    Ok(())
                })?;

                validators.push(Validator::Length {
                    min,
                    max,
                    equal,
                    message,
                });
            }
            "range" => {
                let mut min = None;
                let mut max = None;
                let mut exclusive_min = None;
                let mut exclusive_max = None;
                let mut message = None;

                parse_kv_args(&meta, |key, value| {
                    match key.as_str() {
                        "min" => min = Some(value),
                        "max" => max = Some(value),
                        "exclusive_min" => exclusive_min = Some(value),
                        "exclusive_max" => exclusive_max = Some(value),
                        "message" => {
                            message = Some(expr_to_string(&value)?);
                        }
                        _ => {
                            return Err(syn::Error::new_spanned(
                                &value,
                                format!("unknown range parameter: {}", key),
                            ));
                        }
                    }
                    Ok(())
                })?;

                validators.push(Validator::Range {
                    min,
                    max,
                    exclusive_min,
                    exclusive_max,
                    message,
                });
            }
            "contains" => {
                if meta.input.peek(Token![=]) {
                    let value: MetaNameValue = syn::parse2(quote! { #ident = }.into()).unwrap();
                    // parse `= "..."`
                    meta.input.parse::<Token![=]>()?;
                    let lit: Lit = meta.input.parse()?;
                    let pattern = lit_to_string(&lit)?;
                    validators.push(Validator::Contains {
                        pattern,
                        message: None,
                    });
                    _ = value;
                } else {
                    let mut pattern = String::new();
                    let mut message = None;

                    parse_kv_args(&meta, |key, value| {
                        match key.as_str() {
                            "pattern" => pattern = expr_to_string(&value)?,
                            "message" => message = Some(expr_to_string(&value)?),
                            _ => {
                                return Err(syn::Error::new_spanned(
                                    &value,
                                    format!("unknown contains parameter: {}", key),
                                ));
                            }
                        }
                        Ok(())
                    })?;

                    validators.push(Validator::Contains { pattern, message });
                }
            }
            "does_not_contain" => {
                if meta.input.peek(Token![=]) {
                    meta.input.parse::<Token![=]>()?;
                    let lit: Lit = meta.input.parse()?;
                    let pattern = lit_to_string(&lit)?;
                    validators.push(Validator::DoesNotContain {
                        pattern,
                        message: None,
                    });
                } else {
                    let mut pattern = String::new();
                    let mut message = None;

                    parse_kv_args(&meta, |key, value| {
                        match key.as_str() {
                            "pattern" => pattern = expr_to_string(&value)?,
                            "message" => message = Some(expr_to_string(&value)?),
                            _ => {
                                return Err(syn::Error::new_spanned(
                                    &value,
                                    format!("unknown does_not_contain parameter: {}", key),
                                ));
                            }
                        }
                        Ok(())
                    })?;

                    validators.push(Validator::DoesNotContain { pattern, message });
                }
            }
            "must_match" => {
                let mut other = String::new();
                let mut message = None;

                parse_kv_args(&meta, |key, value| {
                    match key.as_str() {
                        "other" => other = expr_to_string(&value)?,
                        "message" => message = Some(expr_to_string(&value)?),
                        _ => {
                            return Err(syn::Error::new_spanned(
                                &value,
                                format!("unknown must_match parameter: {}", key),
                            ));
                        }
                    }
                    Ok(())
                })?;

                if other.is_empty() {
                    return Err(syn::Error::new(
                        ident.span(),
                        "must_match requires `other` parameter",
                    ));
                }

                validators.push(Validator::MustMatch { other, message });
            }
            "regex" => {
                let mut path = None;
                let mut message = None;

                parse_kv_args(&meta, |key, value| {
                    match key.as_str() {
                        "path" => path = Some(value),
                        "message" => message = Some(expr_to_string(&value)?),
                        _ => {
                            return Err(syn::Error::new_spanned(
                                &value,
                                format!("unknown regex parameter: {}", key),
                            ));
                        }
                    }
                    Ok(())
                })?;

                let path = path.ok_or_else(|| {
                    syn::Error::new(ident.span(), "regex requires `path` parameter")
                })?;

                validators.push(Validator::Regex { path, message });
            }
            "custom" => {
                let mut function: Option<syn::Path> = None;
                let mut message = None;

                parse_kv_args(&meta, |key, value| {
                    match key.as_str() {
                        "function" => {
                            let s = expr_to_string(&value)?;
                            function = Some(syn::parse_str(&s)?);
                        }
                        "message" => message = Some(expr_to_string(&value)?),
                        _ => {
                            return Err(syn::Error::new_spanned(
                                &value,
                                format!("unknown custom parameter: {}", key),
                            ));
                        }
                    }
                    Ok(())
                })?;

                let function = function.ok_or_else(|| {
                    syn::Error::new(ident.span(), "custom requires `function` parameter")
                })?;

                validators.push(Validator::Custom { function, message });
            }
            other => {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("unknown validator: {}", other),
                ));
            }
        }

        Ok(())
    })?;

    Ok(validators)
}

fn parse_optional_message(meta: &syn::meta::ParseNestedMeta) -> syn::Result<Option<String>> {
    if meta.input.peek(syn::token::Paren) {
        let mut message = None;
        parse_kv_args(meta, |key, value| {
            if key == "message" {
                message = Some(expr_to_string(&value)?);
            } else {
                return Err(syn::Error::new_spanned(
                    &value,
                    format!("unexpected parameter: {}", key),
                ));
            }
            Ok(())
        })?;
        Ok(message)
    } else {
        Ok(None)
    }
}

fn parse_kv_args(
    meta: &syn::meta::ParseNestedMeta,
    mut handler: impl FnMut(String, Expr) -> syn::Result<()>,
) -> syn::Result<()> {
    let content;
    syn::parenthesized!(content in meta.input);
    let pairs: Punctuated<MetaNameValue, Token![,]> =
        content.parse_terminated(MetaNameValue::parse, Token![,])?;

    for pair in pairs {
        let key = pair
            .path
            .get_ident()
            .ok_or_else(|| syn::Error::new_spanned(&pair.path, "expected identifier"))?
            .to_string();
        handler(key, pair.value)?;
    }

    Ok(())
}

fn expr_to_string(expr: &Expr) -> syn::Result<String> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Str(s) => Ok(s.value()),
            _ => Err(syn::Error::new_spanned(expr, "expected string literal")),
        },
        _ => Err(syn::Error::new_spanned(expr, "expected string literal")),
    }
}

fn lit_to_string(lit: &Lit) -> syn::Result<String> {
    match lit {
        Lit::Str(s) => Ok(s.value()),
        _ => Err(syn::Error::new_spanned(lit, "expected string literal")),
    }
}

// ---------------------------------------------------------------------------
// Code generation
// ---------------------------------------------------------------------------

fn generate_validation(
    validator: &Validator,
    field_ident: &syn::Ident,
    field_name: &str,
    span: proc_macro2::Span,
) -> syn::Result<TokenStream2> {
    match validator {
        Validator::Email { message } => {
            let msg = error_message(message, "email", "invalid email address");
            Ok(quote_spanned! { span =>
                if !crate::validate::ValidateEmail::validate_email(&self.#field_ident) {
                    errors.add(
                        #field_name,
                        crate::validate::ValidationError::new("email")
                            .with_message(#msg),
                    );
                }
            })
        }
        Validator::Url { message } => {
            let msg = error_message(message, "url", "invalid URL");
            Ok(quote_spanned! { span =>
                if !crate::validate::ValidateUrl::validate_url(&self.#field_ident) {
                    errors.add(
                        #field_name,
                        crate::validate::ValidationError::new("url")
                            .with_message(#msg),
                    );
                }
            })
        }
        Validator::Ip { message } => {
            let msg = error_message(message, "ip", "invalid IP address");
            Ok(quote_spanned! { span =>
                if !crate::validate::ValidateIp::validate_ip(&self.#field_ident) {
                    errors.add(
                        #field_name,
                        crate::validate::ValidationError::new("ip")
                            .with_message(#msg),
                    );
                }
            })
        }
        Validator::NonControlCharacter { message } => {
            let msg = error_message(
                message,
                "non_control_character",
                "contains control characters",
            );
            Ok(quote_spanned! { span =>
                if !crate::validate::ValidateNonControlCharacter::validate_non_control_character(&self.#field_ident) {
                    errors.add(
                        #field_name,
                        crate::validate::ValidationError::new("non_control_character")
                            .with_message(#msg),
                    );
                }
            })
        }
        Validator::Required { message } => {
            let msg = error_message(message, "required", "field is required");
            Ok(quote_spanned! { span =>
                if !crate::validate::ValidateRequired::validate_required(&self.#field_ident) {
                    errors.add(
                        #field_name,
                        crate::validate::ValidationError::new("required")
                            .with_message(#msg),
                    );
                }
            })
        }
        Validator::Length {
            min,
            max,
            equal,
            message,
        } => {
            let msg = error_message(message, "length", "invalid length");
            let min_expr = option_expr(min);
            let max_expr = option_expr(max);
            let equal_expr = option_expr(equal);
            Ok(quote_spanned! { span =>
                if !crate::validate::ValidateLength::validate_length(
                    &self.#field_ident,
                    #min_expr,
                    #max_expr,
                    #equal_expr,
                ) {
                    errors.add(
                        #field_name,
                        crate::validate::ValidationError::new("length")
                            .with_message(#msg),
                    );
                }
            })
        }
        Validator::Range {
            min,
            max,
            exclusive_min,
            exclusive_max,
            message,
        } => {
            let msg = error_message(message, "range", "value out of range");
            let min_expr = option_f64_expr(min);
            let max_expr = option_f64_expr(max);
            let emin_expr = option_f64_expr(exclusive_min);
            let emax_expr = option_f64_expr(exclusive_max);
            Ok(quote_spanned! { span =>
                if !crate::validate::ValidateRange::validate_range(
                    &self.#field_ident,
                    #min_expr,
                    #max_expr,
                    #emin_expr,
                    #emax_expr,
                ) {
                    errors.add(
                        #field_name,
                        crate::validate::ValidationError::new("range")
                            .with_message(#msg),
                    );
                }
            })
        }
        Validator::Contains { pattern, message } => {
            let msg = error_message(message, "contains", &format!("must contain '{}'", pattern));
            Ok(quote_spanned! { span =>
                if !crate::validate::ValidateContains::validate_contains(&self.#field_ident, #pattern) {
                    errors.add(
                        #field_name,
                        crate::validate::ValidationError::new("contains")
                            .with_message(#msg),
                    );
                }
            })
        }
        Validator::DoesNotContain { pattern, message } => {
            let msg = error_message(
                message,
                "does_not_contain",
                &format!("must not contain '{}'", pattern),
            );
            Ok(quote_spanned! { span =>
                if !crate::validate::ValidateDoesNotContain::validate_does_not_contain(&self.#field_ident, #pattern) {
                    errors.add(
                        #field_name,
                        crate::validate::ValidationError::new("does_not_contain")
                            .with_message(#msg),
                    );
                }
            })
        }
        Validator::MustMatch { other, message } => {
            let other_ident = syn::Ident::new(other, span);
            let msg = error_message(
                message,
                "must_match",
                &format!("must match field '{}'", other),
            );
            Ok(quote_spanned! { span =>
                if !crate::validate::validate_must_match(&self.#field_ident, &self.#other_ident) {
                    errors.add(
                        #field_name,
                        crate::validate::ValidationError::new("must_match")
                            .with_message(#msg),
                    );
                }
            })
        }
        Validator::Regex { path, message } => {
            let msg = error_message(message, "regex", "does not match the required pattern");
            Ok(quote_spanned! { span =>
                if !crate::validate::ValidateRegex::validate_regex(&self.#field_ident, &#path) {
                    errors.add(
                        #field_name,
                        crate::validate::ValidationError::new("regex")
                            .with_message(#msg),
                    );
                }
            })
        }
        Validator::Nested => Ok(quote_spanned! { span =>
            errors.merge_self(#field_name, crate::validate::Validate::validate(&self.#field_ident));
        }),
        Validator::Custom { function, message } => {
            let msg = error_message(message, "custom", "custom validation failed");
            Ok(quote_spanned! { span =>
                if let Err(_) = #function(&self.#field_ident) {
                    errors.add(
                        #field_name,
                        crate::validate::ValidationError::new("custom")
                            .with_message(#msg),
                    );
                }
            })
        }
    }
}

fn error_message(custom: &Option<String>, _code: &str, default: &str) -> String {
    custom.clone().unwrap_or_else(|| default.to_string())
}

fn option_expr(expr: &Option<Expr>) -> TokenStream2 {
    match expr {
        Some(e) => quote! { Some(#e as usize) },
        None => quote! { None },
    }
}

fn option_f64_expr(expr: &Option<Expr>) -> TokenStream2 {
    match expr {
        Some(e) => quote! { Some(#e as f64) },
        None => quote! { None },
    }
}
