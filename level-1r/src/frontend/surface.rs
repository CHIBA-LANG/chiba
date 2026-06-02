use std::collections::BTreeMap;

use crate::ast::{DataDecl, MethodReceiver, SourceItem, SourceProgram, TypeDecl};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectSurface {
    pub namespace: String,
    pub imports: Vec<String>,
    pub defs: Vec<SurfaceDef>,
    pub statics: Vec<SurfaceStatic>,
    pub types: Vec<SurfaceType>,
    pub data: Vec<SurfaceData>,
    pub constructors: Vec<SurfaceConstructor>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceDef {
    pub owner: String,
    pub name: String,
    pub receiver: Option<MethodReceiver>,
    pub generics: Vec<String>,
    pub arity: usize,
    pub param_types: Vec<Option<String>>,
    pub return_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceStatic {
    pub owner: String,
    pub name: String,
    pub ty: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceType {
    pub owner: String,
    pub name: String,
    pub generics: Vec<String>,
    pub fields: Vec<SurfaceTypeField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceTypeField {
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceData {
    pub owner: String,
    pub name: String,
    pub generics: Vec<String>,
    pub variants: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceConstructor {
    pub owner: String,
    pub data: String,
    pub name: String,
    pub arity: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceSummary {
    pub namespace: String,
    pub functions: Vec<InterfaceFunction>,
    pub statics: Vec<InterfaceStatic>,
    pub types: Vec<InterfaceType>,
    pub data: Vec<InterfaceData>,
    pub constructors: Vec<InterfaceConstructor>,
    pub imports: Vec<String>,
    pub stable_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceFunction {
    pub symbol: String,
    pub source_name: String,
    pub receiver: Option<MethodReceiver>,
    pub generics: Vec<String>,
    pub arity: usize,
    pub param_types: Vec<Option<String>>,
    pub return_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceStatic {
    pub symbol: String,
    pub ty: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceType {
    pub symbol: String,
    pub generics: Vec<String>,
    pub fields: Vec<InterfaceTypeField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceTypeField {
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceData {
    pub symbol: String,
    pub generics: Vec<String>,
    pub variants: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceConstructor {
    pub symbol: String,
    pub data_symbol: String,
    pub arity: usize,
}

pub fn project_surface(program: &SourceProgram) -> ProjectSurface {
    let namespace = program
        .namespace
        .as_ref()
        .map(|namespace| namespace.dotted())
        .unwrap_or_else(|| "root".to_string());
    let imports = program.imports.iter().map(|import| import.dotted()).collect();
    let defs = program
        .items
        .iter()
        .filter_map(|item| match item {
            SourceItem::Def {
                name,
                receiver,
                generics,
                params,
                return_type,
                ..
            } => Some(SurfaceDef {
                owner: namespace.clone(),
                name: name.clone(),
                receiver: receiver.clone(),
                generics: generics.clone(),
                arity: params.len(),
                param_types: params.iter().map(|param| param.ty.clone()).collect(),
                return_type: return_type.clone(),
            }),
            SourceItem::StaticValue { .. } => None,
        })
        .collect();
    let statics = program
        .items
        .iter()
        .filter_map(|item| match item {
            SourceItem::StaticValue { name, ty, .. } => Some(SurfaceStatic {
                owner: namespace.clone(),
                name: name.clone(),
                ty: ty.clone(),
            }),
            SourceItem::Def { .. } => None,
        })
        .collect();
    let data = program
        .data
        .iter()
        .map(|decl| surface_data(&namespace, decl))
        .collect::<Vec<_>>();
    let types = program
        .types
        .iter()
        .map(|decl| surface_type(&namespace, decl))
        .collect::<Vec<_>>();
    let constructors = program
        .data
        .iter()
        .flat_map(|decl| surface_constructors(&namespace, decl))
        .collect();
    ProjectSurface {
        namespace,
        imports,
        defs,
        statics,
        types,
        data,
        constructors,
    }
}

pub fn build_interface_summary(surface: &ProjectSurface) -> InterfaceSummary {
    let functions = surface
        .defs
        .iter()
        .map(|def| InterfaceFunction {
            symbol: def_symbol(def),
            source_name: def.name.clone(),
            receiver: def.receiver.clone(),
            generics: def.generics.clone(),
            arity: def.arity,
            param_types: def.param_types.clone(),
            return_type: def.return_type.clone(),
        })
        .collect();
    let statics = surface
        .statics
        .iter()
        .map(|static_value| InterfaceStatic {
            symbol: owned_symbol(&static_value.owner, &static_value.name),
            ty: static_value.ty.clone(),
        })
        .collect();
    let data = surface
        .data
        .iter()
        .map(|data| InterfaceData {
            symbol: owned_symbol(&data.owner, &data.name),
            generics: data.generics.clone(),
            variants: data.variants.clone(),
        })
        .collect();
    let types = surface
        .types
        .iter()
        .map(|ty| InterfaceType {
            symbol: owned_symbol(&ty.owner, &ty.name),
            generics: ty.generics.clone(),
            fields: ty
                .fields
                .iter()
                .map(|field| InterfaceTypeField {
                    name: field.name.clone(),
                    ty: field.ty.clone(),
                })
                .collect(),
        })
        .collect();
    let constructors = surface
        .constructors
        .iter()
        .map(|ctor| InterfaceConstructor {
            symbol: format!("{}::{}.{}", ctor.owner, ctor.data, ctor.name),
            data_symbol: owned_symbol(&ctor.owner, &ctor.data),
            arity: ctor.arity,
        })
        .collect();
    let stable_hash = stable_summary_hash(surface);
    InterfaceSummary {
        namespace: surface.namespace.clone(),
        functions,
        statics,
        types,
        data,
        constructors,
        imports: surface.imports.clone(),
        stable_hash,
    }
}

pub fn duplicate_data_names(surface: &ProjectSurface) -> Vec<String> {
    duplicates(surface.data.iter().map(|data| data.name.as_str()))
}

pub fn duplicate_constructor_names(surface: &ProjectSurface) -> Vec<String> {
    let mut by_data = BTreeMap::<String, Vec<&str>>::new();
    for ctor in &surface.constructors {
        by_data
            .entry(format!("{}::{}", ctor.owner, ctor.data))
            .or_default()
            .push(ctor.name.as_str());
    }
    by_data
        .into_values()
        .flat_map(|names| duplicates(names.into_iter()))
        .collect()
}

fn surface_data(owner: &str, decl: &DataDecl) -> SurfaceData {
    SurfaceData {
        owner: owner.to_string(),
        name: decl.name.clone(),
        generics: decl.generics.clone(),
        variants: decl.variant_names(),
    }
}

fn surface_type(owner: &str, decl: &TypeDecl) -> SurfaceType {
    SurfaceType {
        owner: owner.to_string(),
        name: decl.name.clone(),
        generics: decl.generics.clone(),
        fields: decl
            .fields
            .iter()
            .map(|field| SurfaceTypeField {
                name: field.name.clone(),
                ty: field.ty.clone(),
            })
            .collect(),
    }
}

fn surface_constructors(owner: &str, decl: &DataDecl) -> Vec<SurfaceConstructor> {
    decl.variants
        .iter()
        .map(|variant| SurfaceConstructor {
            owner: owner.to_string(),
            data: decl.name.clone(),
            name: variant.name.clone(),
            arity: variant.fields.len(),
        })
        .collect()
}

fn owned_symbol(owner: &str, name: &str) -> String {
    format!("{owner}::{name}")
}

fn def_symbol(def: &SurfaceDef) -> String {
    match &def.receiver {
        Some(receiver) => format!("{}::{}.{}", def.owner, receiver.display_name(), def.name),
        None => owned_symbol(&def.owner, &def.name),
    }
}

fn duplicates<'a>(names: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for name in names {
        *counts.entry(name.to_string()).or_default() += 1;
    }
    counts
        .into_iter()
        .filter_map(|(name, count)| if count > 1 { Some(name) } else { None })
        .collect()
}

fn stable_summary_hash(surface: &ProjectSurface) -> String {
    let mut text = String::new();
    text.push_str(&surface.namespace);
    for import in &surface.imports {
        text.push_str("|use:");
        text.push_str(import);
    }
    for def in &surface.defs {
        text.push_str("|def:");
        if let Some(receiver) = &def.receiver {
            text.push_str(&receiver.display_name());
            text.push('.');
        }
        text.push_str(&def.name);
        text.push('[');
        text.push_str(&def.generics.join(","));
        text.push(']');
        text.push(':');
        text.push_str(&def.arity.to_string());
        text.push('(');
        text.push_str(
            &def.param_types
                .iter()
                .map(|ty| ty.as_deref().unwrap_or("_"))
                .collect::<Vec<_>>()
                .join(","),
        );
        text.push(')');
        text.push_str("->");
        text.push_str(def.return_type.as_deref().unwrap_or("_"));
    }
    for static_value in &surface.statics {
        text.push_str("|static:");
        text.push_str(&static_value.name);
        text.push(':');
        text.push_str(static_value.ty.as_deref().unwrap_or("_"));
    }
    for ty in &surface.types {
        text.push_str("|type:");
        text.push_str(&ty.name);
        text.push('[');
        text.push_str(&ty.generics.join(","));
        text.push(']');
        text.push('{');
        for field in &ty.fields {
            text.push_str(&field.name);
            text.push(':');
            text.push_str(&field.ty);
            text.push(',');
        }
        text.push('}');
    }
    for data in &surface.data {
        text.push_str("|data:");
        text.push_str(&data.name);
        text.push('[');
        text.push_str(&data.generics.join(","));
        text.push(']');
        text.push('{');
        text.push_str(&data.variants.join(","));
        text.push('}');
    }
    for ctor in &surface.constructors {
        text.push_str("|ctor:");
        text.push_str(&ctor.data);
        text.push('.');
        text.push_str(&ctor.name);
        text.push(':');
        text.push_str(&ctor.arity.to_string());
    }
    format!("{:016x}", fnv1a64(text.as_bytes()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
