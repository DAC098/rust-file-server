use chrono::{DateTime, NaiveDateTime, Utc};
use handlebars::{Handlebars, RenderContext, Helper, Context, HelperResult, Output, RenderError};
use lib;

pub fn join_to_path(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    for param in h.params() {
        let value = param.value();

        if value.is_string() {
            let check = value.as_str().unwrap();

            if !check.starts_with("/") {
                out.write("/")?;
            }

            out.write(check.strip_suffix("/").unwrap_or(check))?;
        } else if value.is_number() {
            out.write("/")?;
            out.write(value.to_string().as_str())?
        }
    }

    Ok(())
}

pub fn format_ts_sec(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    let mut datetime = chrono::Utc::now();

    if let Some(param) = h.param(0) {
        let value = param.value();

        if let Some(value) = value.as_i64() {
            datetime = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(value, 0), Utc);
        } else if value.is_null() {
            return Ok(())
        } else {
            return Err(RenderError::new("given timestamp is not a valid i64"));
        }
    }

    if let Some(param) = h.param(1) {
        let value = param.value();

        if let Some(value) = value.as_str() {
            out.write(datetime.format(value).to_string().as_str())?;
        } else {
            return Err(RenderError::new("given format is not a valid string"));
        }
    } else {
        out.write(datetime.to_rfc3339().as_str())?;
    }

    Ok(())
}

pub fn bytes_to_unit(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if let Some(param) = h.param(0) {
        let value = param.value();

        if let Some(value) = value.as_u64() {
            out.write(lib::units::bytes_to_unit(value).as_str())?;
        } else if value.is_null() {
            return Ok(())
        } else {
            return Err(RenderError::new("given timestamp is not a valid u64"));
        }
    }

    Ok(())
}

pub fn value_length(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
    if let Some(param) = h.param(0) {
        let value = param.value();

        if let Some(value) = value.as_array() {
            out.write(value.len().to_string().as_str())?;
        } else if let Some(value) = value.as_str() {
            out.write(value.chars().count().to_string().as_str())?
        } else {
            return Err(RenderError::new("given timestamp is not a valid u64"));
        }
    }

    Ok(())
}