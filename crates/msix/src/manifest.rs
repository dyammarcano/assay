//! AppxManifest.xml parsing. Faithful 1:1 port of the pure-parse surface of Go
//! `pkg/msix` (`msix.go` type tree + `capabilities_xml.go` custom
//! `CapabilitiesBlock.UnmarshalXML`) plus `FormatBytes`.
//!
//! Scope (unit 5a): the manifest *parser* — `parse_appx_manifest` populating
//! Identity / Properties / Dependencies / the ordered `CapabilitiesBlock` /
//! Applications (with `VisualElements`). The zip-backed `Info`/`Extract`/
//! `Verify`/`IsMSIX` surface and the deep `<Extension>` flatten land in 5b.

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;
use serde::Serialize;

/// Bounds capability entries per manifest to mitigate capability-count DoS
/// (T-04-04). Excess entries are dropped and `Truncated` is set. Go
/// `MaxCapabilityEntries`.
pub const MAX_CAPABILITY_ENTRIES: usize = 1024;

// AppxManifest namespace URIs (capabilities_xml.go).
const NS_FOUNDATION: &str = "http://schemas.microsoft.com/appx/manifest/foundation/windows10";
const NS_RESCAP: &str =
    "http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities";
const NS_UAP: &str = "http://schemas.microsoft.com/appx/manifest/uap/windows10";
const NS_UAP2: &str = "http://schemas.microsoft.com/appx/manifest/uap/windows10/2";
const NS_UAP3: &str = "http://schemas.microsoft.com/appx/manifest/uap/windows10/3";
const NS_UAP4: &str = "http://schemas.microsoft.com/appx/manifest/uap/windows10/4";
const NS_UAP6: &str = "http://schemas.microsoft.com/appx/manifest/uap/windows10/6";
const NS_UAP8: &str = "http://schemas.microsoft.com/appx/manifest/uap/windows10/8";
const NS_UAP10: &str = "http://schemas.microsoft.com/appx/manifest/uap/windows10/10";
const NS_UAP13: &str = "http://schemas.microsoft.com/appx/manifest/uap/windows10/13";
const NS_UAP15: &str = "http://schemas.microsoft.com/appx/manifest/uap/windows10/15";

/// Any `<Capability Name="..."/>` element. Go `NamedCap`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct NamedCap {
    pub name: String,
}

/// `<DeviceCapability Name="...">` with optional `<Device>` children. Go `DeviceCap`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DeviceCap {
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub device: Vec<DeviceChild>,
}

/// A `<Device Id="...">` element with optional `<Function>` entries. Go `DeviceChild`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DeviceChild {
    pub id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub function: Vec<DeviceFunc>,
}

/// `<Function Type="..."/>`. Go `DeviceFunc`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DeviceFunc {
    #[serde(rename = "type")]
    pub type_: String,
}

/// Preserves the cross-tag manifest order of capabilities (D-04). Go `OrderedCapRef`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct OrderedCapRef {
    pub namespace: String,
    pub name: String,
    pub index: i32,
}

/// Parsed `<Capabilities>` block with cross-namespace document order preserved.
/// Go `CapabilitiesBlock`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CapabilitiesBlock {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub capability: Vec<NamedCap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uap_capability: Vec<NamedCap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uap2_capability: Vec<NamedCap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uap3_capability: Vec<NamedCap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uap4_capability: Vec<NamedCap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uap6_capability: Vec<NamedCap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uap8_capability: Vec<NamedCap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uap10_capability: Vec<NamedCap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uap13_capability: Vec<NamedCap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub uap15_capability: Vec<NamedCap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub restricted_capability: Vec<NamedCap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub device_capability: Vec<DeviceCap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub custom_capability: Vec<NamedCap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unknown_capability: Vec<NamedCap>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ordered_refs: Vec<OrderedCapRef>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
}

/// `<Identity>` attributes. Go `AppxManifest.Identity`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Identity {
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub processor_architecture: String,
}

/// `<Properties>` child text. Go `AppxManifest.Properties`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Properties {
    pub display_name: String,
    pub description: String,
    pub publisher_display_name: String,
}

/// A `<TargetDeviceFamily>` entry. Go `AppxManifest.Dependencies.TargetDeviceFamily`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TargetDeviceFamily {
    pub name: String,
    pub min_version: String,
    pub max_version_tested: String,
}

/// The parsed-XML mirror of `<uap:VisualElements>`. Go `AppxVisualElementsXML`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AppxVisualElementsXml {
    pub display_name: String,
    pub description: String,
    pub background_color: String,
    pub square150x150_logo: String,
    pub square44x44_logo: String,
}

/// A single `<Application>` node. Go `AppxApplication` (the deep `<Extensions>`
/// tree is parsed in unit 5b).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AppxApplication {
    pub id: String,
    pub executable: String,
    pub entry_point: String,
    pub visual_elements: AppxVisualElementsXml,
}

/// The parsed AppxManifest.xml. Go `AppxManifest` (Applications' `<Extensions>`
/// deep tree deferred to 5b).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AppxManifest {
    pub identity: Identity,
    pub properties: Properties,
    pub target_device_family: Vec<TargetDeviceFamily>,
    pub capabilities: CapabilitiesBlock,
    pub applications: Vec<AppxApplication>,
}

/// Parse raw AppxManifest.xml bytes into a typed [`AppxManifest`]. Wrapped so a
/// malformed manifest never panics; DTDs/entity references are rejected (Go's
/// `encoding/xml` default, matched here by erroring on `<!DOCTYPE>`). Go
/// `ParseAppxManifest`.
pub fn parse_appx_manifest(data: &[u8]) -> Result<AppxManifest, String> {
    let mut reader = NsReader::from_reader(data);
    let mut buf = Vec::new();
    let mut m = AppxManifest::default();
    let mut saw_package = false;

    loop {
        match reader.read_resolved_event_into(&mut buf) {
            Err(e) => return Err(format!("unmarshal appx manifest: {e}")),
            Ok((_, Event::Eof)) => break,
            // encoding/xml rejects DTDs by default (billion-laughs guard).
            Ok((_, Event::DocType(_))) => {
                return Err("unmarshal appx manifest: DOCTYPE not allowed".to_string())
            }
            Ok((_, Event::Start(e))) => {
                let local = local_name(e.name().as_ref());
                match local.as_str() {
                    "Package" => saw_package = true,
                    "Identity" => m.identity = parse_identity(&e),
                    "Properties" => m.properties = parse_properties(&mut reader, &mut buf)?,
                    "Dependencies" => {
                        m.target_device_family = parse_dependencies(&mut reader, &mut buf)?
                    }
                    "Capabilities" => m.capabilities = parse_capabilities(&mut reader, &mut buf)?,
                    "Applications" => m.applications = parse_applications(&mut reader, &mut buf)?,
                    _ => {}
                }
            }
            // ASSAY FIX (diverges from upstream): self-closing elements arrive as `Empty`, not
            // `Start`, and real manifests always write `<Identity ... />` that way — Instagram's
            // does. Without this arm the identity silently parsed as empty.
            //
            // Only attribute-only elements are safe here: `Properties`, `Dependencies`,
            // `Capabilities` and `Applications` delegate to sub-parsers that consume up to
            // their end tag, which does not exist for an empty element.
            Ok((_, Event::Empty(e))) => {
                let local = local_name(e.name().as_ref());
                match local.as_str() {
                    "Package" => saw_package = true,
                    "Identity" => m.identity = parse_identity(&e),
                    _ => {}
                }
            }
            Ok(_) => {}
        }
        buf.clear();
    }

    if !saw_package {
        return Err("unmarshal appx manifest: no <Package> root".to_string());
    }
    Ok(m)
}

/// Local name (strip any `prefix:`) from a raw element/attribute name.
fn local_name(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    match s.rsplit_once(':') {
        Some((_, local)) => local.to_string(),
        None => s.into_owned(),
    }
}

/// Value of the attribute whose local name equals `want` (case-sensitive,
/// matching Go's `attr.Name.Local` comparison).
fn attr(e: &quick_xml::events::BytesStart, want: &str) -> String {
    for a in e.attributes().flatten() {
        if local_name(a.key.as_ref()) == want {
            return a
                .unescape_value()
                .map(|v| v.into_owned())
                .unwrap_or_default();
        }
    }
    String::new()
}

fn parse_identity(e: &quick_xml::events::BytesStart) -> Identity {
    Identity {
        name: attr(e, "Name"),
        version: attr(e, "Version"),
        publisher: attr(e, "Publisher"),
        processor_architecture: attr(e, "ProcessorArchitecture"),
    }
}

fn parse_properties<R: std::io::BufRead>(
    reader: &mut NsReader<R>,
    buf: &mut Vec<u8>,
) -> Result<Properties, String> {
    let mut p = Properties::default();
    let mut cur: Option<String> = None;
    let mut text = String::new();
    loop {
        match reader
            .read_event_into(buf)
            .map_err(|e| format!("properties: {e}"))?
        {
            Event::Start(e) => {
                cur = Some(local_name(e.name().as_ref()));
                text.clear();
            }
            Event::Text(t) => {
                if cur.is_some() {
                    text.push_str(&t.unescape().map_err(|e| format!("text: {e}"))?);
                }
            }
            Event::End(e) => {
                let name = local_name(e.name().as_ref());
                if name == "Properties" {
                    break;
                }
                if cur.as_deref() == Some(name.as_str()) {
                    match name.as_str() {
                        "DisplayName" => p.display_name = text.clone(),
                        "Description" => p.description = text.clone(),
                        "PublisherDisplayName" => p.publisher_display_name = text.clone(),
                        _ => {}
                    }
                    cur = None;
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(p)
}

fn parse_dependencies<R: std::io::BufRead>(
    reader: &mut NsReader<R>,
    buf: &mut Vec<u8>,
) -> Result<Vec<TargetDeviceFamily>, String> {
    let mut out = Vec::new();
    loop {
        match reader
            .read_event_into(buf)
            .map_err(|e| format!("dependencies: {e}"))?
        {
            Event::Start(e) | Event::Empty(e) => {
                if local_name(e.name().as_ref()) == "TargetDeviceFamily" {
                    out.push(TargetDeviceFamily {
                        name: attr(&e, "Name"),
                        min_version: attr(&e, "MinVersion"),
                        max_version_tested: attr(&e, "MaxVersionTested"),
                    });
                }
            }
            Event::End(e) if local_name(e.name().as_ref()) == "Dependencies" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

fn parse_applications<R: std::io::BufRead>(
    reader: &mut NsReader<R>,
    buf: &mut Vec<u8>,
) -> Result<Vec<AppxApplication>, String> {
    let mut out = Vec::new();
    loop {
        match reader
            .read_event_into(buf)
            .map_err(|e| format!("applications: {e}"))?
        {
            Event::Empty(e) if local_name(e.name().as_ref()) == "Application" => {
                out.push(AppxApplication {
                    id: attr(&e, "Id"),
                    executable: attr(&e, "Executable"),
                    entry_point: attr(&e, "EntryPoint"),
                    visual_elements: AppxVisualElementsXml::default(),
                });
            }
            Event::Start(e) if local_name(e.name().as_ref()) == "Application" => {
                let mut app = AppxApplication {
                    id: attr(&e, "Id"),
                    executable: attr(&e, "Executable"),
                    entry_point: attr(&e, "EntryPoint"),
                    visual_elements: AppxVisualElementsXml::default(),
                };
                parse_application_children(reader, buf, &mut app)?;
                out.push(app);
            }
            Event::End(e) if local_name(e.name().as_ref()) == "Applications" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

fn parse_application_children<R: std::io::BufRead>(
    reader: &mut NsReader<R>,
    buf: &mut Vec<u8>,
    app: &mut AppxApplication,
) -> Result<(), String> {
    loop {
        match reader
            .read_event_into(buf)
            .map_err(|e| format!("application: {e}"))?
        {
            Event::Start(e) | Event::Empty(e) => {
                if local_name(e.name().as_ref()) == "VisualElements" {
                    app.visual_elements = AppxVisualElementsXml {
                        display_name: attr(&e, "DisplayName"),
                        description: attr(&e, "Description"),
                        background_color: attr(&e, "BackgroundColor"),
                        square150x150_logo: attr(&e, "Square150x150Logo"),
                        square44x44_logo: attr(&e, "Square44x44Logo"),
                    };
                }
            }
            Event::End(e) if local_name(e.name().as_ref()) == "Application" => break,
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

/// Namespace-URI -> canonical OrderedCapRef tag for `<Capability>` elements.
fn ns_tag(ns: &str) -> Option<&'static str> {
    match ns {
        "" | NS_FOUNDATION => Some(""),
        NS_UAP => Some("uap"),
        NS_UAP2 => Some("uap2"),
        NS_UAP3 => Some("uap3"),
        NS_UAP4 => Some("uap4"),
        NS_UAP6 => Some("uap6"),
        NS_UAP8 => Some("uap8"),
        NS_UAP10 => Some("uap10"),
        NS_UAP13 => Some("uap13"),
        NS_UAP15 => Some("uap15"),
        NS_RESCAP => Some("rescap"),
        _ => None,
    }
}

fn push_capability(c: &mut CapabilitiesBlock, tag: &str, name: &str) {
    let cap = NamedCap {
        name: name.to_string(),
    };
    match tag {
        "" => c.capability.push(cap),
        "uap" => c.uap_capability.push(cap),
        "uap2" => c.uap2_capability.push(cap),
        "uap3" => c.uap3_capability.push(cap),
        "uap4" => c.uap4_capability.push(cap),
        "uap6" => c.uap6_capability.push(cap),
        "uap8" => c.uap8_capability.push(cap),
        "uap10" => c.uap10_capability.push(cap),
        "uap13" => c.uap13_capability.push(cap),
        "uap15" => c.uap15_capability.push(cap),
        "rescap" => c.restricted_capability.push(cap),
        _ => c.unknown_capability.push(cap),
    }
}

/// Custom capability-block walk preserving cross-namespace document order. Go
/// `CapabilitiesBlock.UnmarshalXML`.
fn parse_capabilities<R: std::io::BufRead>(
    reader: &mut NsReader<R>,
    buf: &mut Vec<u8>,
) -> Result<CapabilitiesBlock, String> {
    let mut c = CapabilitiesBlock::default();
    let mut idx: i32 = 0;
    loop {
        let (rr, ev) = reader
            .read_resolved_event_into(buf)
            .map_err(|e| format!("capabilities: {e}"))?;
        let ns = match rr {
            ResolveResult::Bound(n) => String::from_utf8_lossy(n.into_inner()).into_owned(),
            _ => String::new(),
        };
        // Split Start vs Empty so only a Start `<DeviceCapability>` descends into
        // its `<Device>` children (a self-closing one has none).
        let (e, is_start) = match ev {
            Event::End(e) if local_name(e.name().as_ref()) == "Capabilities" => break,
            Event::Eof => break,
            Event::Start(e) => (e, true),
            Event::Empty(e) => (e, false),
            _ => {
                buf.clear();
                continue;
            }
        };

        let local = local_name(e.name().as_ref());
        let name_owned = e.name().as_ref().to_vec();

        if idx >= MAX_CAPABILITY_ENTRIES as i32 {
            c.truncated = true;
            if is_start {
                let mut skip = Vec::new();
                let _ = reader.read_to_end_into(quick_xml::name::QName(&name_owned), &mut skip);
            }
            buf.clear();
            continue;
        }

        match local.as_str() {
            "Capability" => {
                let name = attr(&e, "Name");
                let tag = ns_tag(&ns).map(|t| t.to_string()).unwrap_or_else(|| {
                    if ns.is_empty() {
                        "unknown".to_string()
                    } else {
                        format!("unknown:{ns}")
                    }
                });
                push_capability(&mut c, &tag, &name);
                c.ordered_refs.push(OrderedCapRef {
                    namespace: tag,
                    name,
                    index: idx,
                });
                idx += 1;
            }
            "DeviceCapability" => {
                let name = attr(&e, "Name");
                let mut dc = DeviceCap {
                    name: name.clone(),
                    device: Vec::new(),
                };
                drop(e);
                if is_start {
                    decode_device_children(reader, &name_owned, &mut dc)?;
                }
                c.device_capability.push(dc);
                c.ordered_refs.push(OrderedCapRef {
                    namespace: "device".to_string(),
                    name,
                    index: idx,
                });
                idx += 1;
            }
            "CustomCapability" => {
                let name = attr(&e, "Name");
                c.custom_capability.push(NamedCap { name: name.clone() });
                c.ordered_refs.push(OrderedCapRef {
                    namespace: "custom".to_string(),
                    name,
                    index: idx,
                });
                idx += 1;
            }
            other => {
                let name = attr(&e, "Name");
                if !name.is_empty() && other.ends_with("Capability") {
                    c.unknown_capability.push(NamedCap { name: name.clone() });
                    let tag = if ns.is_empty() {
                        "unknown".to_string()
                    } else {
                        format!("unknown:{ns}")
                    };
                    c.ordered_refs.push(OrderedCapRef {
                        namespace: tag,
                        name,
                        index: idx,
                    });
                    idx += 1;
                }
            }
        }
        buf.clear();
    }
    Ok(c)
}

fn decode_device_children<R: std::io::BufRead>(
    reader: &mut NsReader<R>,
    end_name: &[u8],
    dc: &mut DeviceCap,
) -> Result<(), String> {
    let mut buf = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| format!("device children: {e}"))?
        {
            Event::End(e) if e.name().as_ref() == end_name => break,
            Event::Start(e) if local_name(e.name().as_ref()) == "Device" => {
                let mut child = DeviceChild {
                    id: attr(&e, "Id"),
                    function: Vec::new(),
                };
                let dev_end = e.name().as_ref().to_vec();
                decode_device_functions(reader, &dev_end, &mut child)?;
                dc.device.push(child);
            }
            Event::Empty(e) if local_name(e.name().as_ref()) == "Device" => {
                dc.device.push(DeviceChild {
                    id: attr(&e, "Id"),
                    function: Vec::new(),
                });
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

fn decode_device_functions<R: std::io::BufRead>(
    reader: &mut NsReader<R>,
    end_name: &[u8],
    child: &mut DeviceChild,
) -> Result<(), String> {
    let mut buf = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| format!("device functions: {e}"))?
        {
            Event::End(e) if e.name().as_ref() == end_name => break,
            Event::Start(e) | Event::Empty(e) => {
                if local_name(e.name().as_ref()) == "Function" {
                    child.function.push(DeviceFunc {
                        type_: attr(&e, "Type"),
                    });
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

/// Format a byte count as a human-readable string. Go `FormatBytes`.
pub fn format_bytes(size: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = 1024 * KB;
    const GB: i64 = 1024 * MB;
    if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{size} bytes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foundation_cap() {
        let m = parse_appx_manifest(
            br#"<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10">
  <Identity Name="A" Version="1.0.0.0" Publisher="CN=X"/>
  <Capabilities>
    <Capability Name="internetClient"/>
  </Capabilities>
</Package>"#,
        )
        .unwrap();
        assert_eq!(m.capabilities.capability.len(), 1);
        assert_eq!(m.capabilities.capability[0].name, "internetClient");
        assert_eq!(m.capabilities.ordered_refs.len(), 1);
        assert_eq!(m.capabilities.ordered_refs[0].namespace, "");
    }

    #[test]
    fn uap_cap() {
        let m = parse_appx_manifest(
            br#"<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10">
  <Identity Name="A" Version="1.0.0.0" Publisher="CN=X"/>
  <Capabilities><uap:Capability Name="userDataSystem"/></Capabilities>
</Package>"#,
        )
        .unwrap();
        assert_eq!(m.capabilities.uap_capability.len(), 1);
        assert_eq!(m.capabilities.uap_capability[0].name, "userDataSystem");
        assert_eq!(m.capabilities.ordered_refs[0].namespace, "uap");
    }

    #[test]
    fn uap3_cap() {
        let m = parse_appx_manifest(
            br#"<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap3="http://schemas.microsoft.com/appx/manifest/uap/windows10/3">
  <Identity Name="A" Version="1.0.0.0" Publisher="CN=X"/>
  <Capabilities><uap3:Capability Name="userDataAccountsProvider"/></Capabilities>
</Package>"#,
        )
        .unwrap();
        assert_eq!(m.capabilities.uap3_capability.len(), 1);
        assert_eq!(m.capabilities.ordered_refs[0].namespace, "uap3");
    }

    #[test]
    fn uap2_cap() {
        let m = parse_appx_manifest(
            br#"<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap2="http://schemas.microsoft.com/appx/manifest/uap/windows10/2">
  <Identity Name="A" Version="1.0.0.0" Publisher="CN=X"/>
  <Capabilities><uap2:Capability Name="phoneCallHistorySystem"/></Capabilities>
</Package>"#,
        )
        .unwrap();
        assert_eq!(m.capabilities.uap2_capability.len(), 1);
        assert_eq!(
            m.capabilities.uap2_capability[0].name,
            "phoneCallHistorySystem"
        );
    }

    #[test]
    fn uap8_cap() {
        let m = parse_appx_manifest(
            br#"<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap8="http://schemas.microsoft.com/appx/manifest/uap/windows10/8">
  <Identity Name="A" Version="1.0.0.0" Publisher="CN=X"/>
  <Capabilities><uap8:Capability Name="screenCapture"/></Capabilities>
</Package>"#,
        )
        .unwrap();
        assert_eq!(m.capabilities.uap8_capability.len(), 1);
    }

    #[test]
    fn uap6_cap() {
        let m = parse_appx_manifest(
            br#"<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap6="http://schemas.microsoft.com/appx/manifest/uap/windows10/6">
  <Identity Name="A" Version="1.0.0.0" Publisher="CN=X"/>
  <Capabilities><uap6:Capability Name="backgroundMediaPlayback"/></Capabilities>
</Package>"#,
        )
        .unwrap();
        assert_eq!(m.capabilities.uap6_capability.len(), 1);
    }

    #[test]
    fn rescap() {
        let m = parse_appx_manifest(
            br#"<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities">
  <Identity Name="A" Version="1.0.0.0" Publisher="CN=X"/>
  <Capabilities><rescap:Capability Name="runFullTrust"/></Capabilities>
</Package>"#,
        )
        .unwrap();
        assert_eq!(m.capabilities.restricted_capability.len(), 1);
        assert_eq!(m.capabilities.restricted_capability[0].name, "runFullTrust");
        assert_eq!(m.capabilities.ordered_refs[0].namespace, "rescap");
    }

    #[test]
    fn device_cap() {
        let m = parse_appx_manifest(
            br#"<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10">
  <Identity Name="A" Version="1.0.0.0" Publisher="CN=X"/>
  <Capabilities>
    <DeviceCapability Name="webcam">
      <Device Id="any"><Function Type="name:vendorSpecific"/></Device>
    </DeviceCapability>
  </Capabilities>
</Package>"#,
        )
        .unwrap();
        assert_eq!(m.capabilities.device_capability.len(), 1);
        let dev = &m.capabilities.device_capability[0];
        assert_eq!(dev.name, "webcam");
        assert_eq!(dev.device.len(), 1);
        assert_eq!(dev.device[0].id, "any");
        assert_eq!(dev.device[0].function.len(), 1);
        assert_eq!(dev.device[0].function[0].type_, "name:vendorSpecific");
    }

    #[test]
    fn custom_cap() {
        let m = parse_appx_manifest(
            br#"<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap4="http://schemas.microsoft.com/appx/manifest/foundation/windows10/4">
  <Identity Name="A" Version="1.0.0.0" Publisher="CN=X"/>
  <Capabilities>
    <uap4:CustomCapability Name="microsoft.contoso.somecap_8wekyb3d8bbwe"/>
  </Capabilities>
</Package>"#,
        )
        .unwrap();
        assert_eq!(m.capabilities.custom_capability.len(), 1);
        assert_eq!(m.capabilities.ordered_refs[0].namespace, "custom");
    }

    #[test]
    fn all_namespaces() {
        let m = parse_appx_manifest(
            br#"<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
         xmlns:uap2="http://schemas.microsoft.com/appx/manifest/uap/windows10/2"
         xmlns:uap4="http://schemas.microsoft.com/appx/manifest/foundation/windows10/4"
         xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities">
  <Identity Name="A" Version="1.0.0.0" Publisher="CN=X"/>
  <Capabilities>
    <Capability Name="internetClient"/>
    <uap:Capability Name="userDataSystem"/>
    <uap2:Capability Name="phoneCallHistorySystem"/>
    <rescap:Capability Name="runFullTrust"/>
    <DeviceCapability Name="webcam"/>
    <uap4:CustomCapability Name="contoso.cap_xxxx"/>
  </Capabilities>
</Package>"#,
        )
        .unwrap();
        assert_eq!(m.capabilities.capability.len(), 1);
        assert_eq!(m.capabilities.uap_capability.len(), 1);
        assert_eq!(m.capabilities.uap2_capability.len(), 1);
        assert_eq!(m.capabilities.restricted_capability.len(), 1);
        assert_eq!(m.capabilities.device_capability.len(), 1);
        assert_eq!(m.capabilities.custom_capability.len(), 1);
        assert_eq!(m.capabilities.ordered_refs.len(), 6);
    }

    #[test]
    fn billion_laughs_rejected() {
        let manifest = br#"<?xml version="1.0"?>
<!DOCTYPE Package [
  <!ENTITY a "AAAAAAAAAA">
  <!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;">
]>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10">
  <Identity Name="&b;" Version="1.0.0.0" Publisher="CN=X"/>
</Package>"#;
        assert!(parse_appx_manifest(manifest).is_err());
    }

    #[test]
    fn truncated_xml() {
        assert!(parse_appx_manifest(b"<Package").is_err());
    }

    #[test]
    fn bounded_capabilities() {
        let mut s = String::from(
            r#"<?xml version="1.0"?><Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"><Identity Name="A" Version="1.0.0.0" Publisher="CN=X"/><Capabilities>"#,
        );
        for _ in 0..(MAX_CAPABILITY_ENTRIES + 10) {
            s.push_str(r#"<Capability Name="capx"/>"#);
        }
        s.push_str("</Capabilities></Package>");
        let m = parse_appx_manifest(s.as_bytes()).unwrap();
        assert!(m.capabilities.truncated);
        assert!(m.capabilities.ordered_refs.len() <= MAX_CAPABILITY_ENTRIES);
    }

    #[test]
    fn format_bytes_matches_go() {
        assert_eq!(format_bytes(0), "0 bytes");
        assert_eq!(format_bytes(512), "512 bytes");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
        assert_eq!(format_bytes(1073741824), "1.0 GB");
    }
}
