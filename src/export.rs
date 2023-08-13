use crate::ts::{ExportConfiguration, TsExportError};
use crate::*;
use once_cell::sync::Lazy;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

/// Global type store for collecting custom types to export.
///
/// Populated by `#[ctor]` functions defined in the [`Type`](derive@crate::Type) macro.
pub static TYPES: Lazy<Mutex<(TypeDefs, BTreeSet<ExportError>)>> = Lazy::new(Default::default);

/// Exports all types in the [`TYPES`](static@crate::export::TYPES) map to the provided TypeScript file.
pub fn ts(path: &str) -> Result<(), TsExportError> {
    ts_with_cfg(path, &ExportConfiguration::default())
}

/// Exports all types in the [`TYPES`](static@crate::export::TYPES) map to the provided TypeScript file but allow you to provide a configuration for the exporter.
pub fn ts_with_cfg(path: &str, conf: &ExportConfiguration) -> Result<(), TsExportError> {
    let mut out = "// This file has been generated by Specta. DO NOT EDIT.\n\n".to_string();

    let export_by_default = conf.export_by_default.unwrap_or(true);
    let types = TYPES.lock().expect("Failed to acquire lock on 'TYPES'");

    if let Some(err) = types.1.iter().next() {
        return Err(err.clone().into());
    }

    // We sort by name to detect duplicate types BUT also to ensure the output is deterministic. The SID can change between builds so is not suitable for this.
    let types = types
        .0
        .clone()
        .into_iter()
        .filter(|(_, v)| match v {
            Some(v) => v.export.unwrap_or(export_by_default),
            None => {
                unreachable!("Placeholder type should never be returned from the Specta functions!")
            }
        })
        .collect::<BTreeMap<_, _>>();

    // This is a clone of `detect_duplicate_type_names` but using a `BTreeMap` for deterministic ordering
    let mut map = BTreeMap::new();
    for (sid, dt) in &types {
        match dt {
            Some(dt) => {
                if let Some((existing_sid, existing_impl_location)) =
                    map.insert(dt.name.clone(), (sid, dt.impl_location))
                {
                    if existing_sid != sid {
                        return Err(TsExportError::DuplicateTypeName(
                            dt.name.clone(),
                            dt.impl_location,
                            existing_impl_location,
                        ));
                    }
                }
            }
            None => unreachable!(),
        }
    }

    for (_, typ) in types.iter() {
        out += &ts::export_named_datatype(
            conf,
            match typ {
                Some(v) => v,
                None => unreachable!(),
            },
            &types,
        )?;
        out += "\n\n";
    }

    std::fs::write(path, out).map_err(Into::into)
}
