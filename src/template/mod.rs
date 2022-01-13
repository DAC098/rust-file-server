use std::fmt::Write;
use std::path::PathBuf;
use std::fs::{read_dir, canonicalize};

use handlebars::{Handlebars, TemplateError};

use crate::error;
use crate::config::TemplateConfig;

mod shared_state;
mod helpers;

pub use shared_state::*;

fn recursive_load_directory<'a>(
    config: &TemplateConfig,
    hb: &mut Handlebars,
    directory: &PathBuf,
    template_errors: &mut Vec<TemplateError>,
) -> error::Result<()> {
    for item in read_dir(&directory)? {
        let file = item?;
        let path = file.path();

        if path.is_dir() {
            recursive_load_directory(config, hb, &path, template_errors)?;
        } else if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "hbs" {
                    let name = path.strip_prefix(&config.directory).unwrap();

                    if let Some(name_str) = name.to_str() {
                        if let Err(err) = hb.register_template_file(
                            name_str.strip_suffix(".hbs").unwrap(),
                            &path
                        ) {
                            template_errors.push(err);
                        }
                    } else {
                        println!("path contains invalid unicode characters: {}", name.display());
                    }
                }
            } else {
                println!("failed to determine extension for file: {}", path.display());
            }
        }
    }

    Ok(())
}

pub fn build_registry<'a>(config: TemplateConfig) -> error::Result<Handlebars<'a>> {
    let required_templates = [
        "page/index"
    ];

    let mut hb = Handlebars::new();
    hb.set_dev_mode(config.dev_mode);
    hb.register_helper("join-to-path", Box::new(helpers::join_to_path));
    hb.register_helper("format-ts-sec", Box::new(helpers::format_ts_sec));
    hb.register_helper("bytes-to-unit", Box::new(helpers::bytes_to_unit));
    hb.register_helper("value-length", Box::new(helpers::value_length));

    let mut template_errors: Vec<TemplateError> = Vec::new();
    recursive_load_directory(&config, &mut hb, &config.directory, &mut template_errors)?;

    if let Some(mut path) = config.index_path {
        if path.exists() {
            if !path.is_file() {
                return Err(error::Error::Error("override index path is not a file".into()));
            }

            if path.is_relative() {
                path = canonicalize(path)?;
            }

            if let Err(err) = hb.register_template_file("page/index", &path) {
                template_errors.push(err);
            }
        }
    }

    if template_errors.len() > 0 {
        let mut msg = "there were errors when attempting to load templates:\n".to_owned();

        for err in template_errors {
            msg.write_str(&err.to_string()).unwrap();
            msg.write_str("\n").unwrap();
        }

        return Err(error::Error::Error(msg));
    }

    let mut missing_templates = "the following templates are missing from the registry:\n".to_owned();
    let mut found_missing = false;

    for name in required_templates {
        if !hb.has_template(name) {
            missing_templates.write_str(name).unwrap();
            missing_templates.write_str("\n").unwrap();
            found_missing = true;
        }
    }

    if found_missing {
        return Err(error::Error::Error(missing_templates));
    }

    Ok(hb)
}