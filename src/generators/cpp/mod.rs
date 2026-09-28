use std::borrow::Cow;
use std::path::PathBuf;

mod glaze;
mod gtest;

use atomicow::CowArc;
use itertools::Itertools;
use stringcase::Caser;

use crate::generators::{GeneratedSource, GenerationError, Generator};

use crate::intermediate::{
    ArraySize, DataType, Definition, DefinitionRegistry, IntEnum, Json, JsonField, StringEnum,
};

/// The JSON library to use for serialization and deserialization.
#[derive(Debug, Clone, Copy)]
pub enum JSONLibrary {
    Glaze,
    // Simdjson
}

/// The testing library to use for generating end-to-end roundtrip tests.
#[derive(Debug, Clone, Copy)]
pub enum TestingLibrary {
    GoogleTest,
    // Catch
}

const AUTOGENERATION_NOTICE: &str = "
// This file is auto-generated from a KDL specification by `packet-generator`.
// Please do not modify this, but instead change the original definitions.";

const TAB: &str = "    ";

fn split_documentation(doc: &str, indent_level: usize) -> String {
    super::utils::split_documentation(doc, TAB, "///", indent_level)
}

fn struct_format(string: &str) -> String {
    string.to_pascal_case()
}

fn enum_format(string: &str) -> String {
    string.to_pascal_case()
}

fn enum_variant_format(string: &str) -> String {
    string.to_pascal_case()
}

fn field_format(string: &str) -> String {
    string.to_snake_case()
}

#[derive(Debug)]
pub struct CxxGenerator {
    json_library: JSONLibrary,
    #[expect(
        unused,
        reason = "Round-trip (de)serialiation testing is not implemented yet."
    )]
    testing_library: Option<TestingLibrary>,
    _private: (),
}

impl Default for CxxGenerator {
    fn default() -> Self {
        Self::new(JSONLibrary::Glaze, None)
    }
}

impl CxxGenerator {
    #[must_use]
    pub const fn new(json_library: JSONLibrary, testing_library: Option<TestingLibrary>) -> Self {
        Self {
            json_library,
            testing_library,
            _private: (),
        }
    }
}

impl Generator for CxxGenerator {
    fn generate(
        &self,
        registry: &DefinitionRegistry,
        initial_filename: &str,
    ) -> Result<Vec<GeneratedSource>, GenerationError> {
        let mut content = format!(
            r#"#pragma once

#include <pkgen_helpers.hpp>

{AUTOGENERATION_NOTICE}

"#
        );

        let sorted_definitions = registry
            .sorted_definitions()
            .map_err(GenerationError::CycleFound)?;

        let forward_definitions: Result<Vec<String>, _> = sorted_definitions
            .iter()
            .map(|&def| match registry.get(def) {
                Definition::Json(json) => Ok(format!("struct {};", struct_format(&json.name))),
                Definition::IntEnum(int_enum) => {
                    Ok(format!("enum class {};", enum_format(&int_enum.name)))
                }
                Definition::StringEnum(string_enum) => generate_str_enum_cxx(registry, string_enum),
            })
            .collect();

        content.push_str(&forward_definitions?.join("\n\n"));
        content.push_str("\n\n");

        let generated_sources: Result<Vec<String>, _> = sorted_definitions
            .iter()
            .filter_map(|&def| match registry.get(def) {
                Definition::Json(json) => Some(generate_json_cxx(registry, json)),
                Definition::IntEnum(int_enum) => Some(generate_int_enum_cxx(registry, int_enum)),
                Definition::StringEnum(_string_enum) => None,
            })
            .collect();

        content.push_str(&generated_sources?.join("\n\n"));
        content.push_str("\n\n");

        match self.json_library {
            JSONLibrary::Glaze => {
                content.push_str(glaze::preamble());

                for def in sorted_definitions {
                    if let Definition::Json(json) = registry.get(def) {
                        let content_string = glaze::generate_json_cxx(registry, json)?;
                        content.push_str(&content_string);
                        content.push_str("\n\n");
                    }
                }

                content.push_str(glaze::postamble());
            }
        }

        Ok(vec![GeneratedSource {
            filename: PathBuf::from(format!("{}.hpp", initial_filename)),
            content,
        }])
    }

    fn json_name<'a>(&'a self, definition: &'a Json) -> CowArc<'a, str> {
        CowArc::Owned(struct_format(&definition.name).into())
    }

    fn json_field_name<'a>(&'a self, definition: &'a JsonField) -> CowArc<'a, str> {
        CowArc::Owned(field_format(&definition.name).into())
    }

    fn int_enum_name<'a>(&'a self, definition: &'a IntEnum) -> CowArc<'a, str> {
        CowArc::Owned(enum_format(&definition.name).into())
    }

    fn int_enum_variant_name<'a>(
        &'a self,
        definition: &'a crate::intermediate::IntEnumVariant,
    ) -> CowArc<'a, str> {
        CowArc::Owned(enum_variant_format(&definition.name).into())
    }

    fn string_enum_name<'a>(&'a self, definition: &'a StringEnum) -> CowArc<'a, str> {
        CowArc::Owned(enum_format(&definition.name).into())
    }

    fn string_enum_variant_name<'a>(
        &'a self,
        definition: &'a crate::intermediate::StringEnumVariant,
    ) -> CowArc<'a, str> {
        CowArc::Owned(enum_variant_format(&definition.name).into())
    }
}

/// Converts a `DataType` to types recognized by C++.
fn convert_datatype(
    datatype: &DataType,
    registry: &DefinitionRegistry,
) -> Result<Cow<'static, str>, GenerationError> {
    match datatype {
        DataType::I32 { .. } => Ok(Cow::Borrowed("int32_t")),

        DataType::U32 { .. } => Ok(Cow::Borrowed("uint32_t")),

        DataType::I64 { .. } => Ok(Cow::Borrowed("int64_t")),

        DataType::U64 { .. } => Ok(Cow::Borrowed("uint64_t")),

        // Custom `float`/`double` to support C++20 floating point.
        DataType::F32 { .. } => Ok(Cow::Borrowed("pkg::float32")),
        DataType::F64 => Ok(Cow::Borrowed("pkg::float64")),

        DataType::Bool { .. } => Ok(Cow::Borrowed("bool")),

        DataType::String => Ok(Cow::Borrowed("std::string")),

        DataType::Datetime | DataType::DatetimeUnix => Ok(Cow::Borrowed("pkg::chrono_time")),

        DataType::Map { key, value } => {
            let key = convert_datatype(key, registry)?;
            let value = convert_datatype(value, registry)?;

            Ok(Cow::Owned(format!("std::unordered_map<{key}, {value}>")))
        }

        DataType::Array {
            inner_type,
            size: ArraySize::Dynamic,
        } => {
            let inner = convert_datatype(inner_type, registry)?;

            Ok(Cow::Owned(format!("std::vector<{inner}>")))
        }

        DataType::Array {
            inner_type,
            size: ArraySize::Fixed(size),
        } => {
            let inner = convert_datatype(inner_type, registry)?;

            match size.get() {
                1 => Ok(inner),
                n => Ok(Cow::Owned(format!("std::array<{inner}, {n}>"))),
            }
        }

        DataType::StringArray {
            inner_type,
            separator: _,
            size: ArraySize::Dynamic,
        } => {
            let inner = convert_datatype(inner_type, registry)?;

            Ok(Cow::Owned(format!("pkg::string_list<{inner}>")))
        }

        DataType::StringArray {
            inner_type: _,
            separator: _,
            size: ArraySize::Fixed(_),
        } => {
            // TODO(Arves): Implement this.
            todo!("Fixed array size for string arrays are not implemented.");
        }

        // DataType:: { inner_type } => {
        //     let inner = convert_datatype(inner_type, registry)?;
        //     Ok(inner)
        // }
        DataType::Definition {
            definition: weak,
            // TODO(anri): Handle JSON string encoding
            encoding: _,
        } => {
            let definition = registry.get(*weak);
            match *definition {
                Definition::StringEnum(ref str_enum) => {
                    Ok(Cow::Owned(format!("{}::Type", enum_format(&str_enum.name))))
                }

                Definition::Json(ref json) => Ok(Cow::Owned(struct_format(&json.name))),
                Definition::IntEnum(ref int_enum) => Ok(Cow::Owned(enum_format(&int_enum.name))),
            }
        }

        DataType::Unknown { name: other, .. } => match registry.find(other) {
            Some((definition, _idx)) => match definition {
                Definition::StringEnum(str_enum) => {
                    Ok(Cow::Owned(format!("{}::Type", enum_format(&str_enum.name))))
                }

                Definition::Json(json) => Ok(Cow::Owned(struct_format(&json.name))),
                Definition::IntEnum(int_enum) => Ok(Cow::Owned(enum_format(&int_enum.name))),
            },

            None => Err(GenerationError::TypeNotFound {
                name: other.clone(),
                queried_from: datatype.clone(),
            }),
        },
    }
}

fn generate_json_cxx(
    registry: &DefinitionRegistry,
    json: &Json,
) -> Result<String, GenerationError> {
    // TODO(anri):
    // Calculate the approximate sizes of the C++ types and re-order the fields
    //   to pack them more efficiently, from largest to smallest.
    // We could do this optimization because JSON has no ordering requirement.

    let fields: String = json
        .fields
        .iter()
        .map(|field| -> Result<String, GenerationError> {
            let mut datatype = convert_datatype(&field.type_, registry)?;

            if field.optional {
                datatype = Cow::Owned(format!("std::optional<{datatype}>"));
            }

            let name = field_format(&field.name);
            let doc = split_documentation(&field.doc, 0);

            Ok(format!(
                "
{TAB}{doc}
{TAB}{datatype} {name};"
            ))
        })
        .process_results(|mut x| x.join("\n"))?;

    let struct_name = struct_format(&json.name);
    let struct_doc = split_documentation(&json.doc, 0);

    let content = format!(
        "
{struct_doc}
struct {struct_name} {{
{fields}
}};"
    );

    Ok(content)
}

fn generate_int_enum_cxx(
    _registry: &DefinitionRegistry,
    int_enum: &IntEnum,
) -> Result<String, GenerationError> {
    let start = int_enum.start;

    let mut variants_iter = int_enum.variants.iter().sorted_unstable_by_key(|a| a.index);

    let first_variant = variants_iter
        .next()
        .map(|variant| {
            let name = enum_variant_format(&variant.name);

            let start = variant.value.unwrap_or(start);
            let doc = &variant.doc;

            format!("\n{TAB}/// {doc}\n{TAB}{name} = {start},")
        })
        .unwrap_or_default();

    let variants_str = variants_iter
        .map(|variant| {
            let name = enum_variant_format(&variant.name);
            let maybe_val = variant.value.map(|v| format!(" = {v}")).unwrap_or_default();
            let doc = &variant.doc;

            format!("\n{TAB}/// {doc}\n{TAB}{name}{maybe_val}")
        })
        .join(",\n");

    let doc = split_documentation(&int_enum.doc, 0);

    let content = format!(
        "{doc}
enum class {} {{
{first_variant}
{variants_str}
}};",
        enum_format(&int_enum.name),
    );

    Ok(content)
}

fn generate_str_enum_cxx(
    _registry: &DefinitionRegistry,
    str_enum: &StringEnum,
) -> Result<String, GenerationError> {
    let variants = str_enum
        .variants
        .iter()
        .sorted_unstable_by_key(|a| a.index)
        .map(|variant| {
            let name = enum_variant_format(&variant.name);
            let doc = &variant.doc;
            let val = &variant.value;
            format!("\n{TAB}/// {doc}\n{TAB}constexpr const auto {name} = \"{val}\";")
        })
        .join("\n");

    let doc = split_documentation(&str_enum.doc, 0);

    let content = format!(
        "{doc}
namespace {} {{
{TAB}using Type = std::string;
{variants}
}};",
        enum_format(&str_enum.name),
    );

    Ok(content)
}
