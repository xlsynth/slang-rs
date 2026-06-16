// SPDX-License-Identifier: Apache-2.0

//! Extraction and compatibility support for Slang's serialized AST.
//!
//! Slang releases before v10 generally printed a complete textual type at each
//! use site in `--ast-json`. For example, a port declared with a packed struct
//! typedef could have a `type` field like:
//!
//! ```text
//! struct packed{logic[7:0] data;}bus_t
//! ```
//!
//! Starting in Slang v10, the serializer avoids repeating types that it has
//! already emitted. Later uses are encoded as an address followed by the type's
//! display name, for example `6338692580072 bus_t`. The address identifies an
//! object elsewhere in the same JSON document via that object's `addr` field.
//! Links can also occur inside other textual type definitions, so typedefs may
//! form a chain of references.
//!
//! The parser in [`type_extract`] consumes the older, self-contained textual
//! representation. [`TypeResolver`] bridges the formats by indexing addressable
//! JSON nodes and expanding v10+ links back into that representation before
//! parsing. Older inline types contain no links and pass through unchanged.

use num_bigint::BigInt;
use num_traits::Num;
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, hash_map::Entry};
use std::error::Error;
use std::hash::Hash;
use std::str::FromStr;

mod type_extract;
pub use type_extract::{Field, Range, Type, Variant, parse_type_definition};

#[derive(Debug, PartialEq)]
pub enum PortDir {
    Input,
    Output,
    InOut,
}

#[derive(Debug, PartialEq)]
pub struct Port {
    pub dir: PortDir,
    pub name: String,
    pub ty: Type,
}

pub struct ParameterDef {
    pub name: String,
    pub ty: Type,
}

impl FromStr for PortDir {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "In" => Ok(PortDir::Input),
            "Out" => Ok(PortDir::Output),
            "InOut" => Ok(PortDir::InOut),
            _ => Err(format!("Unsupported I/O direction: {}", s)),
        }
    }
}

pub fn extract_ports(
    cfg: &crate::SlangConfig,
    skip_unsupported: bool,
) -> HashMap<String, Vec<Port>> {
    let result = crate::run_slang(cfg).unwrap();
    extract_ports_from_value(&result, skip_unsupported)
}

struct MemberIter<'a> {
    members: Option<&'a Vec<Value>>,
    kinds: &'a [&'a str],
    current: usize,
}

impl<'a> MemberIter<'a> {
    fn new(value: &'a Value, kinds: &'a [&'a str]) -> Self {
        Self {
            members: value["members"].as_array(),
            kinds,
            current: 0,
        }
    }
}

impl<'a> Iterator for MemberIter<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(members) = self.members {
            while self.current < members.len() {
                let member = &members[self.current];
                let kind = member["kind"].as_str().unwrap();
                self.current += 1;
                if self.kinds.contains(&kind) {
                    return Some(member);
                }
            }
            None
        } else {
            None
        }
    }
}

fn parse_type_definition_no_error(type_str: &str) -> Result<Type, Box<dyn Error>> {
    if type_str == "<error>" {
        Err("Found \"<error>\" type in Slang JSON output.")?;
    }
    parse_type_definition(type_str)
}

fn insert_to_vacant<K, V>(map: &mut HashMap<K, V>, key: K, value: V) -> Result<(), String>
where
    K: Eq,
    K: Hash,
{
    match map.entry(key) {
        Entry::Vacant(entry) => {
            entry.insert(value);
            Ok(())
        }
        Entry::Occupied(_) => Err(String::from("Duplicate insertion")),
    }
}

pub fn extract_ports_from_value(
    value: &Value,
    skip_unsupported: bool,
) -> HashMap<String, Vec<Port>> {
    let mut ports_map = HashMap::new();
    let type_resolver = TypeResolver::new(value);

    for member in MemberIter::new(&value["design"], &["Instance"]) {
        let module_name = member["name"].as_str().unwrap();
        if module_name.is_empty() {
            continue;
        }
        let mut ports = Vec::new();
        let body = type_resolver.resolve_node(&member["body"]);
        for instance_member in MemberIter::new(body, &["Port", "InterfacePort"]) {
            let kind = instance_member["kind"].as_str().unwrap();
            match kind {
                "Port" => {
                    let port_name = instance_member["name"].as_str().unwrap();
                    let direction = instance_member["direction"].as_str().unwrap();
                    let type_str = type_resolver.resolve(instance_member["type"].as_str().unwrap());
                    match parse_type_definition_no_error(&type_str) {
                        Ok(ty) => ports.push(Port {
                            dir: PortDir::from_str(direction).unwrap(),
                            name: port_name.to_string(),
                            ty,
                        }),
                        Err(e) => {
                            if skip_unsupported {
                                continue;
                            } else {
                                panic!("{}", e);
                            }
                        }
                    }
                }
                "InterfacePort" => {
                    if !skip_unsupported {
                        panic!("Interface ports are not currently supported.")
                    }
                }
                _ => continue,
            }
        }
        insert_to_vacant(&mut ports_map, module_name.to_string(), ports)
            .unwrap_or_else(|_| panic!("Duplicate definition of module: {module_name}"));
    }

    ports_map
}

pub fn extract_parameter_defs_from_value(
    value: &Value,
    skip_unsupported: bool,
) -> HashMap<String, Vec<ParameterDef>> {
    let mut parameters_map = HashMap::new();
    let type_resolver = TypeResolver::new(value);

    for member in MemberIter::new(&value["design"], &["Instance"]) {
        let module_name = member["name"].as_str().unwrap();
        if module_name.is_empty() {
            continue;
        }
        let mut parameters = Vec::new();
        let body = type_resolver.resolve_node(&member["body"]);
        for instance_member in MemberIter::new(body, &["Parameter"]) {
            let parameter_name = instance_member["name"].as_str().unwrap();
            let type_str = type_resolver.resolve(instance_member["type"].as_str().unwrap());
            match parse_type_definition_no_error(&type_str) {
                Ok(ty) => parameters.push(ParameterDef {
                    name: parameter_name.to_string(),
                    ty,
                }),
                Err(e) => {
                    if skip_unsupported {
                        continue;
                    } else {
                        panic!("{}", e);
                    }
                }
            }
        }
        insert_to_vacant(&mut parameters_map, module_name.to_string(), parameters)
            .unwrap_or_else(|_| panic!("Duplicate definition of module: {module_name}"));
    }

    parameters_map
}

pub fn extract_parameter_defs(
    cfg: &crate::SlangConfig,
    skip_unsupported: bool,
) -> HashMap<String, Vec<ParameterDef>> {
    let result = crate::run_slang(cfg).unwrap();
    extract_parameter_defs_from_value(&result, skip_unsupported)
}

pub fn extract_modules_from_value(value: &Value) -> Result<Vec<String>, Box<dyn Error>> {
    let definitions = value
        .get("definitions")
        .and_then(|v| v.as_array())
        .ok_or("JSON parsing failed")?;

    let mut modules = Vec::new();

    for definition in definitions {
        if definition.get("kind").and_then(|v| v.as_str()) == Some("Definition")
            && definition.get("definitionKind").and_then(|v| v.as_str()) == Some("Module")
        {
            if let Some(name) = definition.get("name").and_then(|v| v.as_str()) {
                modules.push(name.to_string());
            }
        }
    }

    Ok(modules)
}

pub fn extract_modules(cfg: &crate::SlangConfig) -> Result<Vec<String>, Box<dyn Error>> {
    let result = crate::run_slang(cfg)?;
    extract_modules_from_value(&result)
}

/// Expands the address-based type links emitted by Slang v10 and newer back
/// into the self-contained textual format consumed by this crate's type parser.
///
/// Slang links have the form `<address> <display-name>`, where `<address>`
/// matches an `addr` property elsewhere in the same AST. The display name is
/// significant: it carries the user-visible typedef name, package qualification,
/// and any dimensions applied at the use site. The referenced node contains the
/// underlying definition.
///
/// A resolver borrows the AST because its address index stores references to
/// nodes in that document. Construct one resolver per AST and reuse it for all
/// type fields extracted from that AST.
pub(crate) struct TypeResolver<'a> {
    /// Every JSON object with an `addr` property, keyed by that Slang address.
    by_address: HashMap<u64, &'a Value>,
    /// Finds links both at the start of a type and nested within another type.
    link_pattern: Regex,
}

impl<'a> TypeResolver<'a> {
    /// Builds an address index for all objects reachable from `root`.
    ///
    /// Slang links are not necessarily local to the design node being examined;
    /// a type use can refer to a definition elsewhere in the document. Indexing
    /// the entire document up front makes subsequent resolution deterministic.
    pub(crate) fn new(root: &'a Value) -> Self {
        let mut by_address = HashMap::new();
        Self::index(root, &mut by_address);
        Self {
            by_address,
            link_pattern: Regex::new(
                r"(?P<addr>[0-9]+) (?P<name>[A-Za-z_$][A-Za-z0-9_$:.]*(?:\[[^\]]+\])*(?:\$(?:\[[^\]]+\])*)?)",
            )
            .unwrap(),
        }
    }

    /// Follows an address link to a structured AST node when Slang has replaced
    /// the inline object with a string such as `"123456 module_name"`.
    ///
    /// Instance bodies are shared this way in Slang v11. Inline objects from
    /// older Slang releases are returned unchanged, as are malformed or unknown
    /// links, so callers retain the pre-v11 behavior for those inputs.
    pub(crate) fn resolve_node(&self, value: &'a Value) -> &'a Value {
        let Some(address) = value
            .as_str()
            .and_then(|link| link.split_whitespace().next())
            .and_then(|address| address.parse::<u64>().ok())
        else {
            return value;
        };
        self.by_address.get(&address).copied().unwrap_or(value)
    }

    /// Recursively records addressable objects without making assumptions about
    /// which AST scopes can contain type definitions.
    fn index(value: &'a Value, by_address: &mut HashMap<u64, &'a Value>) {
        match value {
            Value::Object(object) => {
                if let Some(address) = object.get("addr").and_then(Value::as_u64) {
                    by_address.insert(address, value);
                }
                for child in object.values() {
                    Self::index(child, by_address);
                }
            }
            Value::Array(array) => {
                for child in array {
                    Self::index(child, by_address);
                }
            }
            _ => {}
        }
    }

    /// Expands every resolvable Slang link in a textual type definition.
    ///
    /// Resolution is iterative because replacing an outer typedef link can
    /// expose more links inside its target. Unknown addresses and unsupported
    /// node kinds are preserved verbatim so the normal type parser can report
    /// an informative error (or the caller can skip unsupported types).
    pub(crate) fn resolve(&self, type_text: &str) -> String {
        let mut result = type_text.to_string();
        // Cap expansion to guard against malformed or cyclic linked typedefs.
        for _ in 0..64 {
            let replaced = self
                .link_pattern
                .replace_all(&result, |captures: &regex::Captures| {
                    let address = captures["addr"].parse::<u64>().unwrap();
                    self.render_link(address, &captures["name"])
                        .unwrap_or_else(|| captures[0].to_string())
                })
                .into_owned();
            if replaced == result {
                return result;
            }
            result = replaced;
        }
        result
    }

    /// Resolves a `TypeAlias` node that appears directly in a package.
    ///
    /// Direct package members are definitions rather than address-link use
    /// sites, so Slang does not repeat the public alias name in `target`. Pass
    /// that name explicitly to replace any generated aggregate name while
    /// preserving the same behavior as an ordinary linked alias.
    pub(crate) fn resolve_alias(&self, node: &Value, display_name: &str) -> Option<String> {
        let target = self.resolve(node.get("target")?.as_str()?);
        self.render_alias_target(&target, display_name)
    }

    /// Renders one linked definition using the name and dimensions from its use
    /// site rather than the serializer's internal generated name.
    fn render_link(&self, address: u64, display_name: &str) -> Option<String> {
        let node = *self.by_address.get(&address)?;
        match node.get("kind").and_then(Value::as_str)? {
            "TypeAlias" => self.resolve_alias(node, display_name),
            "EnumType" => self.render_enum(node, display_name),
            _ => None,
        }
    }

    /// Applies a typedef's public name and use-site dimensions to its resolved
    /// target text.
    fn render_alias_target(&self, target: &str, display_name: &str) -> Option<String> {
        let display_dimension_start = dimension_start(display_name);
        let display_dimensions = display_dimension_start
            .map(|index| &display_name[index..])
            .unwrap_or("");
        let display_name = display_dimension_start
            .map(|index| &display_name[..index])
            .unwrap_or(display_name);
        if matches!(
            target.split_once('{').map(|(prefix, _)| prefix.trim()),
            Some("struct")
                | Some("struct packed")
                | Some("union")
                | Some("union packed")
                | Some("enum")
        ) {
            // Aggregate targets end in an internal nominal name. Replace it
            // with the package-qualified name from this use site. Dimensions
            // after the target's internal name belong to the typedef itself and
            // must precede dimensions applied at this particular use site.
            let close = target.rfind('}')?;
            let target_suffix = &target[close + 1..];
            let target_dimensions = dimension_start(target_suffix)
                .map(|index| &target_suffix[index..])
                .unwrap_or("");
            Some(format!(
                "{}{display_name}{}",
                &target[..=close],
                merge_dimensions(target_dimensions, display_dimensions)
            ))
        } else {
            let target_dimension_start = dimension_start(target);
            let target_dimensions = target_dimension_start
                .map(|index| &target[index..])
                .unwrap_or("");
            let target = target_dimension_start
                .map(|index| &target[..index])
                .unwrap_or(target);
            Some(format!(
                "{target}{}",
                merge_dimensions(target_dimensions, display_dimensions)
            ))
        }
    }

    /// Reconstructs the legacy textual enum syntax from a v10+ `EnumType` node.
    ///
    /// Unlike struct and union aliases, Slang no longer leaves a complete enum
    /// declaration in a string that can simply be followed. Its members and
    /// values must therefore be rendered from the structured JSON node.
    fn render_enum(&self, node: &Value, display_name: &str) -> Option<String> {
        let members = node.get("members")?.as_array()?;
        let variants = members
            .iter()
            .filter(|member| member.get("kind").and_then(Value::as_str) == Some("EnumValue"))
            .map(|member| {
                let name = member.get("name")?.as_str()?;
                let value = member.get("value")?.as_str()?;
                let (width, signed, magnitude) = parse_integer_literal(value)?;
                let sign = if magnitude.sign() == num_bigint::Sign::Minus {
                    "-"
                } else {
                    ""
                };
                let signed = if signed { "s" } else { "" };
                Some(format!(
                    "{name}={sign}{width}'{signed}d{}",
                    magnitude.magnitude()
                ))
            })
            .collect::<Option<Vec<_>>>()?
            .join(",");
        Some(format!("enum{{{variants}}}{display_name}"))
    }
}

/// Finds the first packed (`[... ]`) or unpacked (`$[...]`) dimension.
///
/// Slang's generated nominal type names can themselves contain `$`, so only
/// `$[` marks an unpacked dimension boundary.
fn dimension_start(type_name: &str) -> Option<usize> {
    match (type_name.find('['), type_name.find("$[")) {
        (Some(packed), Some(unpacked)) => Some(packed.min(unpacked)),
        (Some(packed), None) => Some(packed),
        (None, Some(unpacked)) => Some(unpacked),
        (None, None) => None,
    }
}

/// Combines typedef and use-site dimensions into the parser's canonical form.
///
/// Slang writes a `$` before each independently serialized set of unpacked
/// dimensions. Once nested aliases are expanded there must instead be one `$`
/// separating all packed dimensions from all unpacked dimensions.
fn merge_dimensions(target: &str, display: &str) -> String {
    fn split(dimensions: &str) -> (&str, &str) {
        dimensions
            .find("$[")
            .map(|index| (&dimensions[..index], &dimensions[index + 1..]))
            .unwrap_or((dimensions, ""))
    }

    let (target_packed, target_unpacked) = split(target);
    let (display_packed, display_unpacked) = split(display);
    let unpacked = format!("{target_unpacked}{display_unpacked}");
    let separator = if unpacked.is_empty() { "" } else { "$" };
    format!("{target_packed}{display_packed}{separator}{unpacked}")
}

/// Parses a SystemVerilog integer emitted by Slang.
///
/// The AST can use binary, octal, decimal, or hexadecimal digits, while the
/// existing enum grammar expects decimal text. Signed bit patterns are converted
/// from their fixed-width two's-complement representation before rendering.
/// Slang v11 can also emit unsized enum values as plain decimal strings; these
/// have SystemVerilog's default signed 32-bit integer representation.
fn parse_integer_literal(value: &str) -> Option<(usize, bool, BigInt)> {
    let Some((width, digits)) = value.split_once('\'') else {
        let digits = value.replace('_', "");
        return Some((32, true, BigInt::from_str_radix(&digits, 10).ok()?));
    };
    let (negative, width) = match width.strip_prefix('-') {
        Some(width) => (true, width),
        None => (false, width),
    };
    let width = width.parse::<usize>().ok()?;
    let (signed, radix_char, digits) = if let Some(rest) = digits.strip_prefix('s') {
        (true, rest.chars().next()?, &rest[1..])
    } else {
        (false, digits.chars().next()?, &digits[1..])
    };
    let radix = match radix_char {
        'b' => 2,
        'o' => 8,
        'd' => 10,
        'h' => 16,
        _ => return None,
    };
    let digits = digits.replace('_', "");
    let mut number = BigInt::from_str_radix(&digits, radix).ok()?;
    if negative {
        number = -number;
    } else if signed && width > 0 && number.bit((width - 1) as u64) {
        number -= BigInt::from(1u8) << width;
    }
    Some((width, signed, number))
}

#[cfg(test)]
mod resolver_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolves_slang_v11_typedef_and_enum_links() {
        let ast = json!({
            "nodes": [
                {
                    "name": "bus_t",
                    "kind": "TypeAlias",
                    "addr": 100,
                    "target": "struct packed{logic[7:0] data;logic valid;}generated$1"
                },
                {
                    "name": "color_t",
                    "kind": "EnumType",
                    "addr": 200,
                    "members": [
                        { "name": "RED", "kind": "EnumValue", "value": "2'b0" },
                        { "name": "BLUE", "kind": "EnumValue", "value": "2'b10" }
                    ]
                }
            ]
        });
        let resolver = TypeResolver::new(&ast);

        assert_eq!(
            resolver.resolve("100 bus_t[3:0]"),
            "struct packed{logic[7:0] data;logic valid;}bus_t[3:0]"
        );
        assert_eq!(
            resolver.resolve_alias(&ast["nodes"][0], "pkg::bus_t"),
            Some("struct packed{logic[7:0] data;logic valid;}pkg::bus_t".to_string())
        );
        assert_eq!(
            resolver.resolve("200 pkg::color_t"),
            "enum{RED=2'd0,BLUE=2'd2}pkg::color_t"
        );
    }

    #[test]
    fn resolves_nested_slang_v11_typedef_links() {
        let ast = json!({
            "nodes": [
                {
                    "kind": "TypeAlias",
                    "addr": 100,
                    "target": "struct packed{logic[7:0] data;}generated$1"
                },
                {
                    "kind": "TypeAlias",
                    "addr": 200,
                    "target": "struct packed{100 inner_t value;}generated$2"
                },
                {
                    "kind": "TypeAlias",
                    "addr": 300,
                    "target": "100 inner_t[3:0]"
                },
                {
                    "kind": "TypeAlias",
                    "addr": 400,
                    "target": "logic$[3:0]"
                },
                {
                    "kind": "TypeAlias",
                    "addr": 500,
                    "target": "400 unpacked_t$[1:0]"
                }
            ]
        });
        let resolver = TypeResolver::new(&ast);

        assert_eq!(
            resolver.resolve("200 outer_t"),
            "struct packed{struct packed{logic[7:0] data;}inner_t value;}outer_t"
        );
        assert_eq!(
            resolver.resolve("300 outer_array_t[1:0]"),
            "struct packed{logic[7:0] data;}outer_array_t[3:0][1:0]"
        );
        assert_eq!(resolver.resolve("400 unpacked_t$[1:0]"), "logic$[3:0][1:0]");
        assert!(parse_type_definition(&resolver.resolve("400 unpacked_t$[1:0]")).is_ok());
        assert_eq!(resolver.resolve("500 nested_t"), "logic$[3:0][1:0]");
    }

    #[test]
    fn resolves_slang_v11_dotted_scope_qualified_links() {
        let ast = json!({
            "nodes": [{
                "kind": "TypeAlias",
                "addr": 100,
                "target": "struct packed{logic[7:0] data;}generated$1"
            }]
        });
        let resolver = TypeResolver::new(&ast);
        let resolved = resolver.resolve("100 top.byte_t");

        assert_eq!(resolved, "struct packed{logic[7:0] data;}top.byte_t");
        assert!(parse_type_definition(&resolved).is_ok());
    }

    #[test]
    fn resolves_slang_v11_linked_instance_bodies() {
        let ast = json!({
            "design": {
                "members": [
                    {
                        "kind": "InstanceBody",
                        "addr": 100,
                        "members": [
                            {
                                "kind": "Port",
                                "name": "data",
                                "direction": "In",
                                "type": "logic[7:0]"
                            },
                            {
                                "kind": "Parameter",
                                "name": "WIDTH",
                                "type": "int"
                            }
                        ]
                    },
                    { "kind": "Instance", "name": "top", "body": "100 top" }
                ]
            }
        });

        let ports = extract_ports_from_value(&ast, false);
        assert_eq!(ports["top"][0].name, "data");
        let parameters = extract_parameter_defs_from_value(&ast, false);
        assert_eq!(parameters["top"][0].name, "WIDTH");
    }

    #[test]
    fn resolves_slang_v11_unsized_enum_values() {
        let ast = json!({
            "nodes": [{
                "name": "offset_t",
                "kind": "EnumType",
                "addr": 100,
                "members": [
                    { "name": "ZERO", "kind": "EnumValue", "value": "0" },
                    { "name": "NEGATIVE", "kind": "EnumValue", "value": "-1" },
                    { "name": "NEGATIVE_SIZED", "kind": "EnumValue", "value": "-16'sd1" }
                ]
            }]
        });
        let resolver = TypeResolver::new(&ast);

        assert_eq!(
            resolver.resolve("100 offset_t"),
            "enum{ZERO=32'sd0,NEGATIVE=-32'sd1,NEGATIVE_SIZED=-16'sd1}offset_t"
        );
    }
}
