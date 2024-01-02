use crate::primitives::reference::FileReference;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// use bevy::prelude::*;
use crate::yaml_helpers::parse_unity_yaml;
use anyhow::{bail, Context, Result};

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct UnityMaterial {
    #[serde(alias = "m_Name")]
    pub name: String,

    #[serde(alias = "m_Shader")]
    pub shader: FileReference,

    #[serde(alias = "m_SavedProperties")]
    pub properties: SavedProperties,

    #[serde(default, alias = "stringTagMap")]
    pub string_tags: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SavedProperties {
    #[serde(alias = "serializedVersion")]
    pub serialized_version: u64,

    #[serde(alias = "m_TexEnvs")]
    pub tex_envs: Vec<HashMap<String, TextureInfo>>,

    #[serde(alias = "m_Floats")]
    pub floats: Vec<HashMap<String, f32>>,

    #[serde(alias = "m_Colors")]
    pub colors: Vec<HashMap<String, UnityColor>>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct TextureInfo {
    #[serde(alias = "m_Texture")]
    pub texture: FileReference,
    #[serde(alias = "m_Scale")]
    pub scale: UnityVector2,
    #[serde(alias = "m_Offset")]
    pub offset: UnityVector2,
}

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone)]
pub struct UnityVector2 {
    pub x: f32,
    pub y: f32,
}
#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone)]
pub struct UnityColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

// impl From<UnityColor> for Color {
//     fn from(value: UnityColor) -> Self {
//         Color::Rgba {
//             red: value.r,
//             green: value.g,
//             blue: value.b,
//             alpha: value.a,
//         }
//     }
// }
//
// impl From<&UnityColor> for Color {
//     fn from(value: &UnityColor) -> Self {
//         Color::Rgba {
//             red: value.r,
//             green: value.g,
//             blue: value.b,
//             alpha: value.a,
//         }
//     }
// }

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "object_type")]
enum MaterialContainer {
    Material(UnityMaterial),
    #[serde(other)]
    DontCare,
}

pub fn read_single_material(contents: &str) -> Result<UnityMaterial> {
    let map = parse_unity_yaml(contents)?;

    let (_, output) = map.into_iter().next().context("0 items in material file")?;

    let MaterialContainer::Material(mat) = output else {
        bail!("invalid material file");
    };

    Ok(mat)
}
